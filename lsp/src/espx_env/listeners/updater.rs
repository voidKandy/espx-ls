use crate::{
    embeddings,
    espx_env::agents::inner::InnerAgent,
    store::{database::Database, GlobalStore},
};
use anyhow::anyhow;
use espionox::{
    agents::memory::{Message, MessageRole, MessageStack, ToMessage},
    environment::{
        dispatch::{
            listeners::ListenerMethodReturn, Dispatch, EnvListener, EnvMessage, EnvRequest,
        },
        EnvError, ListenerError,
    },
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
    pub fn init(db_rag: bool) -> Result<Self, EnvError> {
        Ok(Self {
            update: None,
            db_rag,
        })
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

impl EnvListener for RefCountedUpdater {
    fn method<'l>(
        &'l mut self,
        trigger_message: EnvMessage,
        dispatch: &'l mut Dispatch,
    ) -> ListenerMethodReturn {
        Box::pin(async {
            match self.inner_write_lock() {
                Ok(mut wl) => {
                    if let EnvMessage::Request(EnvRequest::GetCompletion { agent_id, .. }) =
                        &trigger_message
                    {
                        if agent_id == InnerAgent::Assistant.id() {
                            if let Some(stack) = wl.update.take() {
                                let agent = dispatch
                                    .get_agent_mut(agent_id)
                                    .map_err(|_| ListenerError::NoAgent)?;
                                clean_message_stack(&mut agent.cache);
                                agent.cache.append(stack);
                            }
                        }
                    }
                }
                Err(err) => {
                    error!("Couln't write lock in listener method method: {:?}", err);
                }
            }
            return Ok(trigger_message);
        })
    }
    fn trigger<'l>(&self, env_message: &'l EnvMessage) -> Option<&'l EnvMessage> {
        match self.inner_read_lock() {
            Ok(rl) => {
                if rl.update.is_none() {
                    return None;
                }

                if let EnvMessage::Request(req) = env_message {
                    if let EnvRequest::GetCompletion { agent_id, .. } = req {
                        if agent_id == &InnerAgent::Assistant.id() {
                            return Some(env_message);
                        }
                    }
                }
            }
            Err(err) => {
                error!("Couln't read lock in listener trigger method: {:?}", err)
            }
        }
        None
    }
}
