use espionox::{
    agents::memory::{MessageRole, OtherRoleTo, ToMessage},
    environment::{
        dispatch::{
            listeners::ListenerMethodReturn, Dispatch, EnvListener, EnvMessage, EnvRequest,
        },
        EnvError, ListenerError,
    },
};
use log::error;

use crate::state::SharedGlobalState;

#[derive(Debug)]
pub struct LRURAG {
    id_of_agent_to_update: String,
    state: SharedGlobalState,
}

impl LRURAG {
    pub fn init(
        id_to_watch: &str,
        // should_trigger: Arc<RwLock<bool>>,
        state: SharedGlobalState,
    ) -> Result<Self, EnvError> {
        Ok(Self {
            id_of_agent_to_update: id_to_watch.to_owned(),
            state,
        })
    }

    fn should_trigger(&self) -> anyhow::Result<bool> {
        Ok(self
            .state
            .get_read()?
            .cache
            .lru
            .should_trigger_listener
            .clone())
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
                let role = MessageRole::Other {
                    alias: "lru_rag".to_owned(),
                    coerce_to: OtherRoleTo::User,
                };
                agent
                    .cache
                    .push(self.state.get_read()?.cache.lru.to_message(role))
            }
            let mut w = self.state.get_write()?;
            w.cache.lru.should_trigger_listener = false;
            Ok(trigger_message)
        })
    }
    fn trigger<'l>(&self, env_message: &'l EnvMessage) -> Option<&'l EnvMessage> {
        if !self
            .should_trigger()
            .map_err(|_| {
                error!("FAILED TO GET TRIGGER WITHIN LISTENER");
            })
            .ok()?
        {
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
