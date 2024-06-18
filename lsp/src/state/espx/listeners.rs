use crate::{
    embeddings,
    handle::buffer_operations::BufferOpChannelSender,
    state::{
        database::{docs::chunks::DBDocumentChunk, Database},
        store::GlobalStore,
    },
};
use anyhow::anyhow;
use espionox::agents::{
    listeners::AgentListener,
    memory::{Message, MessageRole, MessageStack, ToMessage},
};
use std::{ops::DerefMut, sync::Arc};
use tokio::sync::{RwLock, RwLockReadGuard, RwLockWriteGuard};
use tracing::error;

#[derive(Debug)]
pub struct AgentUpdater {
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
    stack.mut_filter_by(lru_role(), false);
    stack.mut_filter_by(database_role(), false);
}

impl Clone for AgentUpdater {
    fn clone(&self) -> Self {
        Self {
            shared_stack: Arc::clone(&self.shared_stack),
            update_from_db: self.update_from_db,
        }
    }
}

impl AgentUpdater {
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

    pub async fn refresh_update_with_similar_database_chunks(
        &mut self,
        db: &Database,
        prompt: &str,
        sender: &mut BufferOpChannelSender,
    ) -> anyhow::Result<()> {
        let emb = embeddings::get_passage_embeddings(vec![prompt])?[0].to_vec();
        let chunks = DBDocumentChunk::get_relavent(db, emb, 0.7).await?;

        sender
            .send_work_done_report(
                Some(&format!(
                    "Found {} relevant chunks in database",
                    chunks.len()
                )),
                None,
            )
            .await?;
        let wl = &mut self.stack_write_lock()?;
        if let Some(ref mut stack) = wl.as_mut() {
            stack.mut_filter_by(database_role(), false);
        }
        for (i, ch) in chunks.iter().enumerate() {
            sender
                .send_work_done_report(
                    Some("Updating Agent memory from Database"),
                    Some((i as f32 / chunks.len() as f32 * 100.0) as u32),
                )
                .await?;
            let message = Message {
                content: ch.to_string(),
                role: database_role(),
            };
            match &mut wl.as_mut() {
                Some(ref mut stack) => {
                    stack.push(message);
                }
                None => *wl.deref_mut() = Some(vec![message].into()),
            }
        }
        sender.send_work_done_end(Some("Finished")).await?;
        Ok(())
    }
}

impl AgentListener for AgentUpdater {
    fn trigger<'l>(&self) -> espionox::agents::listeners::ListenerTrigger {
        "updater".into()
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
