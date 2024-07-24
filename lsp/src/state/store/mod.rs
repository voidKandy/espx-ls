mod burns;
mod docs;
pub mod error;
use self::{
    burns::BurnCache,
    docs::DocLRU,
    error::{StoreError, StoreResult},
};
use super::{
    burns::{Activation, Burn},
    database::{
        models::{thing_to_uri, DBBurn, DBChunk, DBDocument, DBDocumentParams, DatabaseStruct},
        Database,
    },
};
use crate::util::OneOf;
use crate::{
    config::{Config, GLOBAL_CONFIG},
    parsing,
};
use anyhow::anyhow;
pub use docs::walk_dir;
use espionox::agents::memory::{Message, ToMessage};
use lsp_types::{TextDocumentContentChangeEvent, Uri};
use tracing::{debug, info, instrument, warn};

#[derive(Debug)]
pub struct GlobalStore {
    docs: DocLRU,
    pub burns: BurnCache,
    pub db: Option<DatabaseStore>,
}

pub(super) const CACHE_SIZE: usize = 5;

#[derive(Debug)]
pub struct DatabaseStore {
    pub client: Database,
    pub cache: Vec<DBDocument>,
}

impl ToMessage for GlobalStore {
    fn to_message(&self, role: espionox::agents::memory::MessageRole) -> Message {
        let mut whole_message = String::from("Here are the most recently accessed documents: ");
        for (uri, doc_text) in self.docs.0.into_iter() {
            whole_message.push_str(&format!(
                "[BEGINNNING OF DOCUMENT: {}]\n{}\n[END OF DOCUMENT: {}]\n",
                uri.as_str(),
                doc_text,
                uri.as_str()
            ));
        }
        debug!("LRU CACHE COERCED TO MESSAGE: {}", whole_message);

        Message {
            role,
            content: whole_message,
        }
    }
}
impl DatabaseStore {
    pub async fn read_all_docs_to_cache(&mut self) -> StoreResult<()> {
        self.cache = DBDocument::get_all(&self.client).await?;
        Ok(())
    }
}

impl GlobalStore {
    pub async fn from_config(cfg: &Config) -> Self {
        let db = match &cfg.database {
            Some(db_cfg) => match Database::init(db_cfg).await {
                Ok(db) => Some(DatabaseStore {
                    client: db,
                    cache: vec![],
                }),
                Err(err) => {
                    debug!(
                        "PROBLEM INTIALIZING DATABASE IN STATE, RETURNING NONE. ERROR: {:?}",
                        err
                    );
                    None
                }
            },
            None => None,
        };
        Self {
            docs: DocLRU::default(),
            burns: BurnCache::default(),
            db,
        }
    }

    /// Currently wipes whole database with anything matching the cache. Not the best course of
    /// action i think, but works for now
    #[tracing::instrument(name = "updating database from store", skip_all)]
    pub async fn try_update_database(&mut self) -> StoreResult<()> {
        match self.db.as_ref() {
            None => return Err(StoreError::new_not_present("No database connection")),
            Some(db) => {
                let mut all_to_create = vec![];
                for (uri, text) in self.all_docs() {
                    if db.cache.iter().any(|d| &d.uri == uri) {
                        DBDocument::take_by_field(&db.client, "uri", &uri).await?;
                        DBChunk::take_by_field(&db.client, "uri", &uri).await?;
                        DBBurn::take_by_field(&db.client, "uri", &uri).await?;
                    }
                    all_to_create.push(DBDocumentParams::build(text, uri.clone())?);
                }
                DBDocument::create_many(&db.client, all_to_create).await?;
                Ok(())
            }
        }
    }

    #[tracing::instrument(name = "updating store from database")]
    pub async fn try_update_from_database(&mut self) -> StoreResult<()> {
        match self.db.as_mut() {
            None => return Err(StoreError::new_not_present("No database connection")),
            Some(db) => {
                db.read_all_docs_to_cache().await?;
                // for now we'll just get the top cache size amount, this should be
                // implemented differently down the road
                for dbdoc in db.cache.clone().into_iter().take(CACHE_SIZE) {
                    let uri = thing_to_uri(&dbdoc.id)?;
                    let burns = dbdoc.burns;
                    let text = dbdoc
                        .chunks
                        .iter()
                        .fold(String::new(), |acc, ch| format!("{}\n{}", acc, ch.content));
                    self.update_doc(&text, uri.clone());
                    for b in burns {
                        self.burns.insert_burn(uri.clone(), b.burn);
                    }
                }
                Ok(())
            }
        }
    }

    pub fn docs_at_capacity(&self) -> bool {
        self.docs.0.at_capacity()
    }

    pub fn all_docs(&self) -> Vec<(&Uri, &str)> {
        self.docs
            .0
            .read_all()
            .into_iter()
            .map(|(k, v)| (k, v.as_str()))
            .collect()
    }

    /// This should be used very sparingly as it completely circumvents the utility of an LRU
    pub fn read_doc(&self, uri: &Uri) -> StoreResult<String> {
        self.docs
            .0
            .read(uri)
            .ok_or(StoreError::new_not_present(uri.as_str()))
    }

    pub fn get_doc(&mut self, uri: &Uri) -> StoreResult<String> {
        self.docs
            .0
            .get(uri)
            .ok_or(StoreError::new_not_present(uri.as_str()))
    }

    #[instrument(name = "update doc and burns from change notification", skip_all)]
    pub fn update_doc_and_burns_from_lsp_change_notification(
        &mut self,
        change: &TextDocumentContentChangeEvent,
        uri: Uri,
    ) -> StoreResult<()> {
        let text = self.get_doc(&uri)?;
        let mut burns_to_insert = Vec::<Burn>::new();
        debug!("updating docs");
        self.docs.update_with_change(&text, uri.clone(), change)?;
        let change_range = change.range.ok_or(anyhow!("Range should be some"))?;

        let mut parsed_burns = Burn::all_in_text(&change.text);

        if parsed_burns.is_empty() {
            return Ok(());
        }
        if let Some(burns) = self.burns.take_burns_in_range(
            &uri,
            change.range.expect("failed to get range from change event"),
        ) {
            debug!("got {} burns in change range", burns.len());
            for mut burn in burns {
                if let Some(mut matching_parsed_variant) = parsed_burns
                    .iter()
                    .position(|pb| pb.activation.matches_variant(&burn.activation))
                    .and_then(|i| Some(parsed_burns.remove(i)))
                {
                    let parsed_matches_cached_pos = match matching_parsed_variant.activation {
                        Activation::Multi(ref mut a) => {
                            let start_range = a.start_range.as_mut();
                            start_range.start.line += change_range.start.line;
                            start_range.start.character += change_range.start.character;
                            start_range.end.line += change_range.start.line;
                            start_range.end.character += change_range.start.character;

                            let end_range = a.end_range.as_mut();
                            end_range.start.line += change_range.start.line;
                            end_range.start.character += change_range.start.character;
                            end_range.end.line += change_range.start.line;
                            end_range.end.character += change_range.start.character;

                            let cached_range = burn.activation.range();
                            let cached_range = cached_range.peek_right().unwrap();
                            matching_parsed_variant
                                .activation
                                .overlaps(cached_range.0.as_ref())
                                || matching_parsed_variant
                                    .activation
                                    .overlaps(cached_range.1.as_ref())
                        }
                        Activation::Single(ref mut a) => {
                            let range = a.range.as_mut();
                            range.start.line += change_range.start.line;
                            range.start.character += change_range.start.character;
                            range.end.line += change_range.start.line;
                            range.end.character += change_range.start.character;

                            let cached_range = burn.activation.range();
                            let cached_range = cached_range.peek_left().unwrap();
                            matching_parsed_variant
                                .activation
                                .overlaps(cached_range.as_ref())
                        }
                    };

                    if parsed_matches_cached_pos {
                        burn.update_activation(matching_parsed_variant)?;
                        burns_to_insert.push(burn);
                    } else {
                        burns_to_insert.push(matching_parsed_variant)
                    }
                }
            }
        }
        burns_to_insert.append(&mut parsed_burns);
        debug!("inserting burns: {:?}", burns_to_insert);
        burns_to_insert.into_iter().for_each(|b| {
            self.burns.insert_burn(uri.clone(), b);
        });

        Ok(())
    }

    // Maybe a function could be written to see if two burns are likely the same
    pub fn update_burns_on_doc(&mut self, uri: &Uri) -> StoreResult<()> {
        let text = self.read_doc(uri)?;
        let mut prev_burns_opt = self.burns.take_burns_on_doc(uri);
        for new_burn in Burn::all_in_text(&text) {
            match prev_burns_opt.as_mut().and_then(|bv| {
                bv.iter()
                    .position(|b| {
                        b.activation.matches_variant(&new_burn.activation) && {
                            let range = b.activation.range();
                            match range.peek_left() {
                                Some(range) => new_burn.activation.overlaps(range.as_ref()),
                                None => {
                                    let ranges = range.peek_right().unwrap();
                                    new_burn.activation.overlaps(ranges.0.as_ref())
                                        || new_burn.activation.overlaps(ranges.1.as_ref())
                                }
                            }
                        }
                    })
                    .and_then(|i| Some(bv.remove(i)))
            }) {
                Some(b) => self.burns.insert_burn(uri.clone(), b),
                None => self.burns.insert_burn(uri.clone(), new_burn),
            }
        }
        Ok(())
    }

    #[tracing::instrument(name = "updating document", skip_all)]
    pub fn update_doc(&mut self, text: &str, uri: Uri) {
        self.docs.0.update(uri, text.to_owned());
    }
}
