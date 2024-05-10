mod burns;
pub mod database;
pub mod error;
mod tests;
mod updater;
use self::database::{docs::FullDBDocument, Database};
use crate::{config::GLOBAL_CONFIG, util::LRUCache};
use burns::BurnCache;
use error::{StoreError, StoreResult};
use espionox::agents::memory::{Message, MessageRole, OtherRoleTo, ToMessage};
use log::{debug, info};
use lsp_types::Url;
pub use updater::{walk_dir, AssistantUpdater};

#[derive(Debug)]
pub struct GlobalStore {
    pub docs: DocLRU,
    pub burns: BurnCache,
    pub updater: AssistantUpdater,
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

#[derive(Debug)]
pub struct DocLRU(pub(super) LRUCache<Url, String>);
impl Default for DocLRU {
    fn default() -> Self {
        Self(LRUCache::new(5))
    }
}

impl DatabaseStore {
    pub async fn read_all_docs_to_cache(&mut self) -> anyhow::Result<()> {
        let docs = self.client.get_all_docs().await?;
        self.cache = docs;
        Ok(())
    }
}

pub(super) const AMT_CHANGES_TO_TRIGGER_UPDATE: usize = 5;
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
            updater: AssistantUpdater::default(),
            db,
        }
    }

    pub fn docs_at_capacity(&self) -> bool {
        self.docs.0.at_capacity()
    }

    pub fn get_doc(&mut self, url: &Url) -> Option<String> {
        self.docs.0.get(url)
    }

    pub fn update_doc(&mut self, text: &str, url: Url) {
        self.docs.0.update(url, text.to_owned());
        info!("SENT UPDATE TO STATIC CACHE");
        self.increment_quick_agent_updates_counter()
    }

    pub fn update_quick_agent(&mut self) {
        let role = MessageRole::Other {
            alias: "LRU".to_owned(),
            coerce_to: OtherRoleTo::User,
        };
        *self
            .updater
            .quick
            .message
            .write()
            .expect("Couldn't write lock listener_update") = Some(self.to_message(role));
    }

    pub async fn update_db_rag_agent(&mut self) -> StoreResult<()> {
        let mut db_update_write = self
            .updater
            .db
            .message
            .write()
            .expect("Couldn't write lock database update");
        if db_update_write.is_none() {
            if let Some(db) = &self.db {
                let role = MessageRole::Other {
                    alias: String::from("DATABASE"),
                    coerce_to: espionox::agents::memory::OtherRoleTo::System,
                };
                let content = db
                    .cache
                    .iter()
                    .map(|d| d.to_string())
                    .collect::<Vec<String>>()
                    .join("\n");
                let message = Message { role, content };
                *db_update_write = Some(message);
            }
        }
        return Ok(());
    }

    pub fn increment_quick_agent_updates_counter(&mut self) {
        info!("UPDATING LRU CHANGES COUNTER");
        let should_trigger = self.updater.quick.message.read().unwrap().is_some();
        self.updater.quick.counter += 1;
        if self.updater.quick.counter >= AMT_CHANGES_TO_TRIGGER_UPDATE && !should_trigger {
            self.update_quick_agent();
            self.updater.quick.counter = 0;
        }
    }
}
