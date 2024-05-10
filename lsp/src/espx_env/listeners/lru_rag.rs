use std::sync::{Arc, RwLock};

use anyhow::anyhow;
use espionox::{
    agents::memory::Message,
    environment::{
        dispatch::{
            listeners::ListenerMethodReturn, Dispatch, EnvListener, EnvMessage, EnvRequest,
        },
        EnvError, ListenerError,
    },
};
use log::{error, info};

use crate::espx_env::agents::inner::InnerAgent;

#[derive(Debug)]
pub struct LRURAG {
    lru_update: Arc<RwLock<Option<Message>>>,
}

impl LRURAG {
    pub fn init(lru_update: Arc<RwLock<Option<Message>>>) -> Result<Self, EnvError> {
        Ok(Self { lru_update })
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
                        if agent_id == InnerAgent::QuickAssistant.id() {
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
                        .get_agent_mut(&InnerAgent::QuickAssistant.id())
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
                if agent_id == &InnerAgent::QuickAssistant.id() {
                    error!("LRU SHOULD UPDATE AGENT");
                    return Some(env_message);
                }
            }
        }
        None
    }
}
