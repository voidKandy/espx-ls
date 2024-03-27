use std::sync::{Arc, RwLock};

use espionox::{
    agents::memory::{MessageRole, OtherRoleTo, ToMessage},
    environment::{
        dispatch::{
            listeners::ListenerMethodReturn, Dispatch, EnvListener, EnvMessage, EnvRequest,
        },
        EnvError, ListenerError,
    },
};

use crate::cache::GLOBAL_CACHE;

#[derive(Debug)]
pub struct LRURAG {
    id_of_agent_to_update: String,
    should_trigger: Arc<RwLock<bool>>,
}

impl LRURAG {
    pub fn init(id_to_watch: &str, should_trigger: Arc<RwLock<bool>>) -> Result<Self, EnvError> {
        Ok(Self {
            id_of_agent_to_update: id_to_watch.to_owned(),
            should_trigger,
        })
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
                let agent = dispatch
                    .get_agent_mut(&self.id_of_agent_to_update)
                    .map_err(|_| ListenerError::NoAgent)?;
                let cache = GLOBAL_CACHE.read().unwrap();
                let role = MessageRole::Other {
                    alias: "lru_rag".to_owned(),
                    coerce_to: OtherRoleTo::User,
                };
                agent.cache.push(cache.lru.to_message(role))
            }
            *self.should_trigger.write().unwrap() = false;
            Ok(trigger_message)
        })
    }
    fn trigger<'l>(&self, env_message: &'l EnvMessage) -> Option<&'l EnvMessage> {
        if !*self.should_trigger.read().unwrap() {
            return None;
        }
        if let EnvMessage::Request(req) = env_message {
            if let EnvRequest::GetCompletion { agent_id, .. } = req {
                if agent_id == &self.id_of_agent_to_update {
                    return Some(env_message);
                }
            }
        }
        None
    }
}
