pub mod burns;
pub mod database;
pub mod error;
pub mod espx;
pub mod store;
use anyhow::anyhow;
use burns::{Activation, Burn};
use database::{
    error::DatabaseError,
    models::{
        DBBurn, DBBurnParams, DBChunk, DBChunkParams, DatabaseStruct, FieldQuery, QueryBuilder,
    },
    Database,
};
use error::StateError;
use espionox::{agents::Agent, prelude::MessageRole};
use lsp_types::{TextDocumentContentChangeEvent, Uri};
use std::{collections::HashMap, sync::Arc};
use store::GlobalStore;
use tokio::sync::{RwLock, RwLockReadGuard, RwLockWriteGuard};
use tracing::{debug, warn};

use espx::EspxEnv;

use crate::config::{Config, DatabaseConfig, GLOBAL_CONFIG};

use self::error::StateResult;

#[derive(Debug)]
pub struct GlobalState {
    pub store: GlobalStore,
    pub espx_env: EspxEnv,
    pub database: Option<database::Database>,
}

#[derive(Debug)]
pub struct SharedGlobalState(Arc<RwLock<GlobalState>>);

impl SharedGlobalState {
    pub async fn init() -> anyhow::Result<Self> {
        Ok(Self(Arc::new(RwLock::new(
            GlobalState::init(&GLOBAL_CONFIG).await?,
        ))))
    }
}

impl Clone for SharedGlobalState {
    fn clone(&self) -> Self {
        Self(Arc::clone(&self.0))
    }
}

impl SharedGlobalState {
    pub fn get_read(&self) -> anyhow::Result<RwLockReadGuard<'_, GlobalState>> {
        match self.0.try_read() {
            Ok(g) => Ok(g),
            Err(e) => Err(e.into()),
        }
    }

    pub fn get_write(&mut self) -> anyhow::Result<RwLockWriteGuard<'_, GlobalState>> {
        match self.0.try_write() {
            Ok(g) => Ok(g),
            Err(e) => Err(e.into()),
        }
    }
}

impl GlobalState {
    #[tracing::instrument(name = "initializing global state")]
    pub async fn init(config: &Config) -> StateResult<Self> {
        let store = GlobalStore::from_config(&config);
        let database = Database::init(
            config
                .database
                .as_ref()
                .unwrap_or(&DatabaseConfig::default()),
        )
        .await
        .ok();
        let espx_env = EspxEnv::init().await?;
        Ok(Self {
            store,
            espx_env,
            database,
        })
    }

    /// Currently wipes whole database with anything matching the cache. Not the best course of
    /// action i think, but works for now
    #[tracing::instrument(name = "updating database from store", skip_all)]
    pub async fn try_update_database(&mut self) -> StateResult<()> {
        match self.database.as_ref() {
            None => return Err(StateError::DBNotPresent),
            Some(db) => {
                let mut q = QueryBuilder::begin();

                for (uri, text) in self.store.all_docs() {
                    q.push(&DBChunk::delete(FieldQuery::new("uri", uri)?)?);
                    q.push(&DBBurn::delete(FieldQuery::new("uri", uri)?)?);
                    for params in DBChunkParams::from_text(uri.clone(), text)? {
                        q.push(&DBChunk::create(params)?);
                    }

                    if let Some(burns) = self.store.burns.read_burns_on_doc(uri) {
                        for burn in burns {
                            let params = DBBurnParams::from_burn(burn.clone(), uri.clone());
                            q.push(&DBBurn::create(params)?);
                        }
                    }
                }
                debug!("running update query");
                db.client
                    .query(q.end())
                    .await
                    .expect("failed to query transaction");
                Ok(())
            }
        }
    }

    #[tracing::instrument(name = "updating store from database", skip_all)]
    pub async fn try_update_from_database(&mut self) -> StateResult<()> {
        match self.database.as_mut() {
            None => return Err(StateError::DBNotPresent),
            Some(db) => {
                let burns: Vec<DBBurn> = db
                    .client
                    .select(DBBurn::db_id())
                    .await
                    .map_err(|err| DatabaseError::from(err))?;

                let chunks: Vec<DBChunk> = db
                    .client
                    .select(DBChunk::db_id())
                    .await
                    .map_err(|err| DatabaseError::from(err))?;

                debug!("adding burns to store: {:?} ", burns);
                for burn in burns {
                    self.store.burns.insert_burn(burn.uri, burn.burn)
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
                    self.store.update_doc(&content, uri);
                }
                Ok(())
            }
        }
    }

    pub fn update_conversation_file(&mut self, agent: &Agent) -> StateResult<()> {
        let mut out_string_vec = vec![];
        for message in agent.cache.as_ref().into_iter() {
            let role_str = {
                if let MessageRole::Other { alias, .. } = &message.role {
                    alias.to_string()
                } else {
                    message.role.to_string()
                }
            };
            let role_str = convert_ascii(&role_str, '𝐀');
            out_string_vec.push(format!("# {}\n\n", &role_str));

            for chunk in message.content.split(". ") {
                out_string_vec.push(chunk.to_owned());
            }
        }
        let content_to_write = out_string_vec.join("\n");
        warn!("updating conversation file: {}", content_to_write);
        std::fs::write(GLOBAL_CONFIG.conversation_file(), content_to_write).unwrap();
        return Ok(());
    }

    // making this a tracing instrument caused a stack overflow
    pub fn update_doc_from_lsp_change_notification(
        &mut self,
        change: &TextDocumentContentChangeEvent,
        uri: Uri,
    ) -> StateResult<()> {
        let text = self.store.get_doc(&uri)?;
        let range = change
            .range
            .ok_or(anyhow!("no range in change notification"))?;

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

        self.store.update_doc(&lines.join("\n"), uri);
        Ok(())
    }

    #[tracing::instrument(name = "update burns from change notification", skip_all)]
    pub fn update_burns_from_lsp_change_notification(
        &mut self,
        change: &TextDocumentContentChangeEvent,
        uri: Uri,
    ) -> StateResult<()> {
        let mut burns_to_insert = Vec::<Burn>::new();
        let change_range = change.range.ok_or(anyhow!("Range should be some"))?;

        let mut parsed_burns = Burn::all_in_text(&change.text);

        if parsed_burns.is_empty() {
            return Ok(());
        }
        if let Some(burns) = self.store.burns.take_burns_in_range(
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
            self.store.burns.insert_burn(uri.clone(), b);
        });

        Ok(())
    }
}

// For making the role look 𝐍 𝐈 𝐂 𝐄
fn convert_ascii(str: &str, target: char) -> String {
    let start_code_point = target as u32;
    let str = str.to_lowercase();
    let mut chars = vec![' '];
    str.chars().for_each(|c| {
        let offset = c as u32 - 'a' as u32;
        chars.push(std::char::from_u32(start_code_point + offset).unwrap_or(c));
        chars.push(' ');
    });

    chars.into_iter().collect()
}
