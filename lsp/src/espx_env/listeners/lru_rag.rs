use std::sync::{Arc, RwLock};

use anyhow::anyhow;
use espionox::{
    agents::memory::{Message, MessageRole, OtherRoleTo},
    environment::{
        dispatch::{
            listeners::{self, ListenerMethodReturn},
            Dispatch, EnvListener, EnvMessage, EnvRequest,
        },
        EnvError, ListenerError,
    },
};
use log::{error, info};

use crate::{cache::GlobalCache, state::SharedGlobalState};

#[derive(Debug)]
pub struct LRURAG {
    id_of_agent_to_update: String,
    lru_update: Arc<RwLock<Option<Message>>>,
}

impl LRURAG {
    pub fn init(
        id_to_watch: &str,
        lru_update: Arc<RwLock<Option<Message>>>,
    ) -> Result<Self, EnvError> {
        // let lru_update = state.get_read()?.cache.lru.listener_update.clone();
        Ok(Self {
            id_of_agent_to_update: id_to_watch.to_owned(),
            lru_update,
        })
    }

    pub fn role() -> MessageRole {
        MessageRole::Other {
            alias: "rag".to_owned(),
            coerce_to: OtherRoleTo::User,
        }
    }
}

impl EnvListener for LRURAG {
    fn method<'l>(
        &'l mut self,
        trigger_message: EnvMessage,
        dispatch: &'l mut Dispatch,
    ) -> ListenerMethodReturn {
        Box::pin(async {
            if let Some(_) = match &trigger_message {
                EnvMessage::Request(req) => match req {
                    EnvRequest::GetCompletion { agent_id, .. } => {
                        if agent_id == &self.id_of_agent_to_update {
                            Some(())
                        } else {
                            None
                        }
                    }
                    _ => None,
                },
                _ => None,
            } {
                if let Some(message) = self
                    .lru_update
                    .write()
                    .map_err(|e| {
                        anyhow!("LRU LISTENER COULD NOT GET WRITE LOCK ON UPDATE: {:?}", e)
                    })?
                    .take()
                {
                    info!("UPDATING AGENT CONTEXT WITH LRU");
                    let agent = dispatch
                        .get_agent_mut(&self.id_of_agent_to_update)
                        .map_err(|_| ListenerError::NoAgent)?;
                    agent.cache.push(message);
                }
            }
            Ok(trigger_message)
        })
    }
    fn trigger<'l>(&self, env_message: &'l EnvMessage) -> Option<&'l EnvMessage> {
        if self.lru_update.read().ok()?.is_none() {
            error!("LRU SHOULD NOT UPDATE AGENT");
            return None;
        }
        if let EnvMessage::Request(req) = env_message {
            if let EnvRequest::GetCompletion { agent_id, .. } = req {
                if agent_id == &self.id_of_agent_to_update {
                    error!("LRU SHOULD UPDATE AGENT");
                    return Some(env_message);
                }
            }
        }
        None
    }
}
