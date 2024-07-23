use anyhow::anyhow;
use espionox::agents::{
    listeners::AgentListener,
    memory::{Message, MessageRole, MessageStack},
};
use std::{ops::DerefMut, sync::Arc};
use tokio::sync::{RwLock, RwLockReadGuard, RwLockWriteGuard};
use tracing::error;

#[derive(Debug)]
pub struct AgentRagUpdater {
    shared_stack: Arc<RwLock<Option<MessageStack>>>,
    update_from_db: bool,
}

pub fn database_role() -> MessageRole {
    MessageRole::Other {
        alias: "DATABASE".to_string(),
        coerce_to: espionox::agents::memory::OtherRoleTo::System,
    }
}

pub fn lru_role() -> MessageRole {
    MessageRole::Other {
        alias: "LRU".to_string(),
        coerce_to: espionox::agents::memory::OtherRoleTo::System,
    }
}

/// Before updating agent cache, we need to remove the LRU and DATABASE role messages in order
/// to keep context under control
fn clean_message_stack(stack: &mut MessageStack) {
    stack.mut_filter_by(&lru_role(), false);
    stack.mut_filter_by(&database_role(), false);
}

impl Clone for AgentRagUpdater {
    fn clone(&self) -> Self {
        Self {
            shared_stack: Arc::clone(&self.shared_stack),
            update_from_db: self.update_from_db,
        }
    }
}

impl AgentRagUpdater {
    pub fn init(update_from_db: bool) -> Self {
        Self {
            shared_stack: Arc::new(RwLock::new(None)),
            update_from_db,
        }
    }
    pub fn stack_write_lock(&self) -> anyhow::Result<RwLockWriteGuard<'_, Option<MessageStack>>> {
        match self.shared_stack.try_write() {
            Ok(lock) => Ok(lock),
            Err(err) => Err(anyhow!("Failed to write lock updater: {:?}", err)),
        }
    }

    pub fn stack_read_lock(&self) -> anyhow::Result<RwLockReadGuard<'_, Option<MessageStack>>> {
        match self.shared_stack.try_read() {
            Ok(lock) => Ok(lock),
            Err(err) => Err(anyhow!("Failed to read lock updater: {:?}", err)),
        }
    }
}

impl AgentListener for AgentRagUpdater {
    fn trigger<'l>(&self) -> espionox::agents::listeners::ListenerTrigger {
        "rag".into()
    }
    fn sync_method<'l>(
        &'l mut self,
        _a: &'l mut espionox::agents::Agent,
    ) -> espionox::agents::error::AgentResult<()> {
        match self.stack_write_lock() {
            Ok(mut wl) => {
                if let Some(stack) = wl.take() {
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
