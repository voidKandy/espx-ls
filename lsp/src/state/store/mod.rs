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
        error::DatabaseError,
        models::{
            DBBurn, DBBurnParams, DBChunk, DBChunkParams, DatabaseStruct, FieldQuery, QueryBuilder,
        },
        Database,
    },
};
use crate::config::Config;
use anyhow::anyhow;
pub use docs::walk_dir;
use espionox::agents::memory::{Message, ToMessage};
use lsp_types::{TextDocumentContentChangeEvent, Uri};
use std::collections::HashMap;
use tracing::{debug, instrument, warn};

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
    // pub cache: Vec<DBDocument>,
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

impl GlobalStore {
    pub async fn from_config(cfg: &Config) -> Self {
        let db = match &cfg.database {
            Some(db_cfg) => match Database::init(db_cfg).await {
                Ok(db) => Some(DatabaseStore { client: db }),
                Err(err) => {
                    debug!(
                        "problem intializing database in state, returning none. error: {:?}",
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
                let mut q = QueryBuilder::begin();

                for (uri, text) in self.all_docs() {
                    q.push(&DBChunk::delete(FieldQuery::new("uri", uri)?)?);
                    q.push(&DBBurn::delete(FieldQuery::new("uri", uri)?)?);
                    for params in DBChunkParams::from_text(uri.clone(), text)? {
                        q.push(&DBChunk::create(params)?);
                    }

                    if let Some(burns) = self.burns.read_burns_on_doc(uri) {
                        for burn in burns {
                            let params = DBBurnParams::from_burn(burn.clone(), uri.clone());
                            q.push(&DBBurn::create(params)?);
                        }
                    }
                }
                debug!("running update query");
                db.client
                    .client
                    .query(q.end())
                    .await
                    .expect("failed to query transaction");
                Ok(())
            }
        }
    }

    #[tracing::instrument(name = "updating store from database", skip_all)]
    pub async fn try_update_from_database(&mut self) -> StoreResult<()> {
        match self.db.as_mut() {
            None => return Err(StoreError::new_not_present("No database connection")),
            Some(db) => {
                let burns: Vec<DBBurn> = db
                    .client
                    .client
                    .select(DBBurn::db_id())
                    .await
                    .map_err(|err| StoreError::from(DatabaseError::from(err)))?;

                let chunks: Vec<DBChunk> = db
                    .client
                    .client
                    .select(DBChunk::db_id())
                    .await
                    .map_err(|err| StoreError::from(DatabaseError::from(err)))?;

                debug!("adding burns to store: {:?} ", burns);
                for burn in burns {
                    self.burns.insert_burn(burn.uri, burn.burn)
                }

                debug!("adding chunks to store: {:?} ", chunks);
                for (uri, content) in chunks.into_iter().fold(HashMap::new(), |mut map, ch| {
                    match map.get_mut(&ch.uri) {
                        Some(content) => *content = format!("{} {}", content, ch.content),
                        None => {
                            let _ = map.insert(ch.uri, ch.content);
                        }
                    }
                    map
                }) {
                    self.docs.0.update(uri, content);
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

    // making this a tracing instrument caused a stack overflow
    pub fn update_doc_from_lsp_change_notification(
        &mut self,
        change: &TextDocumentContentChangeEvent,
        uri: Uri,
    ) -> StoreResult<()> {
        let text = self.get_doc(&uri)?;
        let range = change
            .range
            .ok_or(StoreError::from(anyhow!("no range in change notification")))?;

        let mut lines: Vec<String> = text.lines().map(|l| l.to_string()).collect();
        let change_lines: Vec<String> = change.text.lines().map(|l| l.to_string()).collect();

        if range.start.line as usize >= lines.len()
            || range.start.line as usize + change_lines.len() >= lines.len()
        {
            return Err(
                anyhow!("why does the change start line exceed the available lines?").into(),
            );
        }

        for (i, cl) in change_lines.into_iter().enumerate() {
            let line = &mut lines[range.start.line as usize + i];
            for (k, b) in cl.as_bytes().into_iter().enumerate() {
                let change_idx = if i == 0 {
                    range.start.character as usize + k
                } else {
                    k
                };
                if change_idx >= line.len() {
                    for _ in 0..(change_idx - line.len()) + 1 {
                        line.push(' ');
                    }
                }
                let line_as_bytes = unsafe { line.as_bytes_mut() };
                line_as_bytes[change_idx] = *b;
            }
        }

        self.update_doc(&lines.join("\n"), uri);
        Ok(())
    }

    #[instrument(name = "update burns from change notification", skip_all)]
    pub fn update_burns_from_lsp_change_notification(
        &mut self,
        change: &TextDocumentContentChangeEvent,
        uri: Uri,
    ) -> StoreResult<()> {
        let mut burns_to_insert = Vec::<Burn>::new();
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
