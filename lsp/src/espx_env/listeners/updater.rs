use crate::{
    embeddings,
    store::{database::Database, GlobalStore},
};
use anyhow::anyhow;
use espionox::agents::{
    listeners::AgentListener,
    memory::{Message, MessageRole, MessageStack, ToMessage},
};
use log::error;
use std::sync::Arc;
use tokio::sync::{RwLock, RwLockReadGuard, RwLockWriteGuard};

#[derive(Debug)]
pub struct AssistantUpdater {
    update: Option<MessageStack>,
    db_rag: bool,
}

#[derive(Debug)]
pub struct RefCountedUpdater(Arc<RwLock<AssistantUpdater>>);

impl Clone for RefCountedUpdater {
    fn clone(&self) -> Self {
        Self(Arc::clone(&self.0))
    }
}

impl RefCountedUpdater {
    pub fn inner_write_lock(&self) -> anyhow::Result<RwLockWriteGuard<'_, AssistantUpdater>> {
        match self.0.try_write() {
            Ok(lock) => Ok(lock),
            Err(err) => Err(anyhow!("Failed to write lock updater: {:?}", err)),
        }
    }

    pub fn inner_read_lock(&self) -> anyhow::Result<RwLockReadGuard<'_, AssistantUpdater>> {
        match self.0.try_read() {
            Ok(lock) => Ok(lock),
            Err(err) => Err(anyhow!("Failed to read lock updater: {:?}", err)),
        }
    }
}

impl From<AssistantUpdater> for RefCountedUpdater {
    fn from(value: AssistantUpdater) -> Self {
        Self(Arc::new(RwLock::new(value)))
    }
}

fn database_role() -> MessageRole {
    MessageRole::Other {
        alias: "DATABASE".to_string(),
        coerce_to: espionox::agents::memory::OtherRoleTo::System,
    }
}

fn lru_role() -> MessageRole {
    MessageRole::Other {
        alias: "LRU".to_string(),
        coerce_to: espionox::agents::memory::OtherRoleTo::System,
    }
}

/// Before updating agent cache, we need to remove the LRU and DATABASE role messages in order
/// to keep context under control
fn clean_message_stack(stack: &mut MessageStack) {
    stack.mut_filter_by(lru_role(), false);
    stack.mut_filter_by(database_role(), false);
}

impl AssistantUpdater {
    pub fn init(db_rag: bool) -> Self {
        Self {
            update: None,
            db_rag,
        }
    }
    pub async fn refresh_update_with_similar_database_chunks(
        &mut self,
        db: &Database,
        prompt: &str,
    ) -> anyhow::Result<()> {
        let emb = embeddings::get_passage_embeddings(vec![prompt])?[0].to_vec();
        let chunks = db.get_relavent_chunks(emb, 0.7).await?;
        if let Some(ref mut stack) = &mut self.update {
            stack.mut_filter_by(database_role(), false);
        }
        for ch in chunks {
            let message = Message {
                content: ch.to_string(),
                role: database_role(),
            };
            match &mut self.update {
                Some(ref mut stack) => {
                    stack.push(message);
                }
                None => self.update = Some(vec![message].into()),
            }
        }
        Ok(())
    }

    pub async fn refresh_update_with_cache(&mut self, store: &GlobalStore) -> anyhow::Result<()> {
        let message = store.to_message(lru_role());
        match &mut self.update {
            Some(ref mut stack) => {
                stack.mut_filter_by(lru_role(), false);
                stack.push(message);
            }
            None => self.update = Some(vec![message].into()),
        }
        Ok(())
    }
}

impl AgentListener for RefCountedUpdater {
    fn trigger<'l>(&self) -> espionox::agents::listeners::ListenerTrigger {
        "updater".into()
    }
    fn sync_method<'l>(
        &'l mut self,
        _a: &'l mut espionox::agents::Agent,
    ) -> espionox::agents::error::AgentResult<()> {
        match self.inner_write_lock() {
            Ok(mut wl) => {
                if let Some(stack) = wl.update.take() {
                    clean_message_stack(&mut _a.cache);
                    _a.cache.append(stack);
                }
            }
            Err(err) => {
                error!("Couln't write lock in listener method method: {:?}", err);
            }
        }
        Ok(())
    }
}
