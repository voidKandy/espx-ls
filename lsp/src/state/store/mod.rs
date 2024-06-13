mod burns;
mod docs;
pub mod error;
mod lru;
mod tests;
use self::{
    docs::DocLRU,
    error::{StoreError, StoreResult},
};
use super::database::{docs::FullDBDocument, Database};
use crate::config::GLOBAL_CONFIG;
use anyhow::anyhow;
use burns::BurnCache;
pub use docs::{update_text_with_change, walk_dir};
use espionox::agents::memory::{Message, ToMessage};
use futures::AsyncWriteExt;
use lru::*;
use lsp_types::{TextDocumentContentChangeEvent, Uri};
use nom::AsChar;
use std::{fs, path::PathBuf};
use tracing::{debug, error, info};

#[derive(Debug)]
pub struct GlobalStore {
    docs: DocLRU,
    pub burns: BurnCache,
    pub db: Option<DatabaseStore>,
}

#[derive(Debug)]
pub struct DatabaseStore {
    pub client: Database,
    pub cache: Vec<FullDBDocument>,
}

impl ToMessage for GlobalStore {
    fn to_message(&self, role: espionox::agents::memory::MessageRole) -> Message {
        let mut whole_message = String::from("Here are the most recently accessed documents: ");
        for (url, doc_text) in self.docs.0.into_iter() {
            whole_message.push_str(&format!(
                "[BEGINNNING OF DOCUMENT: {}]\n{}\n[END OF DOCUMENT: {}]\n",
                url.as_str(),
                doc_text,
                url.as_str()
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
    pub async fn read_all_docs_to_cache(&mut self) -> anyhow::Result<()> {
        let docs = self.client.get_all_docs().await?;
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

    pub fn docs_at_capacity(&self) -> bool {
        self.docs.0.at_capacity()
    }

    pub fn get_doc(&mut self, url: &Uri) -> Option<String> {
        self.docs.0.get(url)
    }

    pub fn update_doc_from_lsp_change_notification(
        &mut self,
        change: &TextDocumentContentChangeEvent,
        url: Uri,
    ) -> StoreResult<()> {
        // should fail gracefully
        let text = self.docs.0.get(&url).ok_or(StoreError::NotPresent)?;
        let new_text = update_text_with_change(&text, change)?;

        self.docs.0.update(url, new_text);
        Ok(())
        // self.increment_quick_agent_updates_counter()
    }

    pub fn update_doc(&mut self, text: &str, url: Uri) {
        self.docs.0.update(url, text.to_owned());
        // self.increment_quick_agent_updates_counter()
    }
}
