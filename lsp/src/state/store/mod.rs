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
    burns::{all_activations_in_text, Burn, BurnActivation, MultiLineBurn, SingleLineBurn},
    database::{
        models::{burns::DBDocumentBurn, full::FullDBDocument, DatabaseStruct},
        Database,
    },
};
use crate::{config::GLOBAL_CONFIG, parsing};
use anyhow::anyhow;
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
        let docs = FullDBDocument::get_all(&self.client).await?;
        self.cache = docs
            .into_iter()
            .map(|d| Into::<FullDBDocument>::into(d))
            .collect();
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

    #[tracing::instrument(name = "updating database from store")]
    pub async fn try_update_database(&mut self) -> StoreResult<()> {
        match self.db.as_ref() {
            None => return Err(StoreError::new_not_present("No database connection")),
            Some(db) => {
                let mut all_db_docs = vec![];
                let all_docs = self.all_docs().clone();
                debug!("got {} docs from cache", all_docs.len());
                for (uri, text) in all_docs {
                    let mut all_burns = vec![];
                    if let Some(burns) = self.burns.read_burns_on_doc(uri) {
                        for (line, activation) in burns {
                            all_burns.push(DBDocumentBurn::new(
                                uri.clone(),
                                vec![*line],
                                activation.clone(),
                            ));
                        }
                    }
                    all_db_docs.push(FullDBDocument::new(text, uri.clone(), all_burns)?);
                }
                FullDBDocument::insert_or_update_many(&db.client, all_db_docs).await?;
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
                    let burns = dbdoc.burns;
                    let text = dbdoc
                        .chunks
                        .iter()
                        .fold(String::new(), |acc, ch| format!("{}\n{}", acc, ch.content));
                    self.update_doc(&text, dbdoc.id.clone());
                    for b in burns {
                        for l in b.lines {
                            self.burns
                                .insert_burn(dbdoc.id.clone(), l, b.activation.clone());
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

    #[tracing::instrument(name = "updating burns on document", skip(self))]
    pub async fn update_burns_on_doc(&mut self, uri: &Uri) -> StoreResult<()> {
        let text = self.get_doc(&uri)?;
        // i may come to regret this, but i think if we just remove all the burns and then reparse
        // them that will make sure we don't 'leak' any burns
        if let Some(b) = self.burns.take_burns_on_doc(uri) {
            warn!("removed burns: {:?}", b);
        }
        if let Some(db) = &self.db {
            DBDocumentBurn::take_all(&db.client).await?;
        }

        let activations = all_activations_in_text(&text);

        for (lines, activation) in activations {
            if !lines.is_empty() {
                for l in lines.iter() {
                    self.burns.insert_burn(uri.clone(), *l, activation.clone());
                }

                if let Some(db) = &self.db {
                    DBDocumentBurn::new(uri.clone(), lines, activation)
                        .insert(&db.client)
                        .await?;
                }
            }
        }

        Ok(())
    }

    #[tracing::instrument(name = "updating document", skip(self))]
    pub fn update_doc(&mut self, text: &str, uri: Uri) {
        self.docs.0.update(uri, text.to_owned());
    }
}
