mod burns;
mod docs;
pub mod error;
mod lru;
mod tests;
use self::{
    burns::BurnCache,
    docs::DocLRU,
    error::{StoreError, StoreResult},
};
use super::{
    burns::{Burn, BurnActivation, MultiLineBurn, SingleLineBurn},
    database::{
        docs::{burns::DBDocumentBurn, FullDBDocument},
        Database,
    },
};
use crate::{config::GLOBAL_CONFIG, parsing};
pub use docs::{update_text_with_change, walk_dir};
use espionox::agents::memory::{Message, ToMessage};
use lsp_types::{TextDocumentContentChangeEvent, Uri};
use tracing::{debug, warn};

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
    pub cache: Vec<FullDBDocument>,
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
        let docs = FullDBDocument::get_all_docs(&self.client).await?;
        self.cache = docs;
        Ok(())
    }
}

impl GlobalStore {
    pub async fn init() -> Self {
        let db = match &GLOBAL_CONFIG.database {
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

    pub async fn try_update_from_database(&mut self) -> StoreResult<()> {
        match self.db.as_mut() {
            None => return Err(StoreError::new_not_present("No database connection")),
            Some(db) => {
                db.read_all_docs_to_cache().await?;
                // for now we'll just get the top cache size amount, this should be
                // implemented differently down the road
                for dbdoc in db.cache.clone().into_iter().take(CACHE_SIZE) {
                    let uri = dbdoc.info.uri;
                    let burns = dbdoc.burns;
                    let text = dbdoc.chunks.into_text();
                    self.update_doc(&text, uri.clone());
                    for b in burns {
                        // something is fishy here...
                        for l in b.lines {
                            self.burns.insert_burn(uri.clone(), l, b.activation.clone());
                        }
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

    pub fn update_doc_from_lsp_change_notification(
        &mut self,
        change: &TextDocumentContentChangeEvent,
        uri: Uri,
    ) -> StoreResult<()> {
        let text = self.get_doc(&uri)?;
        let new_text = update_text_with_change(&text, change)?;
        self.docs.0.update(uri, new_text);
        Ok(())
    }

    pub fn update_burns_on_doc(&mut self, uri: &Uri) -> StoreResult<()> {
        let text = self.get_doc(&uri)?;
        // i may come to regret this, but i think if we just remove all the burns and then reparse
        // them that will make sure we don't 'leak' any burns
        let _ = self.burns.take_burns_on_doc(uri);

        for burn in SingleLineBurn::all_variants() {
            let mut lines = parsing::all_lines_with_pattern(&burn.trigger_string(), &text);
            lines.append(&mut parsing::all_lines_with_pattern(
                &burn.echo_content(),
                &text,
            ));

            if !lines.is_empty() {
                for l in lines.iter() {
                    self.burns
                        .insert_burn(uri.clone(), *l, BurnActivation::Single(burn.clone()));
                }

                if let Some(db) = &self.db {
                    let dbburn =
                        DBDocumentBurn::from(uri, lines, BurnActivation::Single(burn.clone()));
                    dbburn.insert(&db.client);
                }
            } else {
                warn!(
                    "No singleline burn of variant: {:?} found on doc: {}",
                    burn,
                    uri.as_str()
                );
            }
        }

        for burn in MultiLineBurn::all_variants() {
            let lines_and_chars =
                parsing::all_lines_with_pattern_with_char_positions(&burn.trigger_string(), &text);

            if !lines_and_chars.is_empty() {
                for (l, _) in lines_and_chars.iter() {
                    self.burns
                        .insert_burn(uri.clone(), *l, BurnActivation::Multi(burn.clone()));
                }

                if let Some(db) = &self.db {
                    let dbburn = DBDocumentBurn::from(
                        uri,
                        lines_and_chars.iter().map(|(l, _)| *l).collect(),
                        BurnActivation::Multi(burn.clone()),
                    );
                    dbburn.insert(&db.client);
                }
            } else {
                warn!(
                    "No multiline burn of variant: {:?} found on doc: {}",
                    burn,
                    uri.as_str()
                );
            }
        }

        Ok(())
    }

    #[tracing::instrument(name = "updating document", skip(self))]
    pub fn update_doc(&mut self, text: &str, uri: Uri) {
        self.docs.0.update(uri, text.to_owned());
    }
}
