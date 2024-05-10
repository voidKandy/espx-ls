use crate::espx_env::agents::inner::InnerAgent;
use espionox::{
    agents::memory::{Message, MessageRole, OtherRoleTo},
    environment::{
        dispatch::{
            listeners::ListenerMethodReturn, Dispatch, EnvListener, EnvMessage, EnvNotification,
            EnvRequest,
        },
        EnvError, ListenerError,
    },
};
use log::error;
use std::sync::{Arc, RwLock};

#[derive(Debug)]
pub struct DBRAG {
    db_update: Arc<RwLock<Option<Message>>>,
}

impl DBRAG {
    pub fn init(db_update: Arc<RwLock<Option<Message>>>) -> Result<Self, EnvError> {
        Ok(Self { db_update })
    }

    pub fn role() -> MessageRole {
        MessageRole::Other {
            alias: "db_rag".to_owned(),
            coerce_to: OtherRoleTo::User,
        }
    }
}

impl EnvListener for DBRAG {
    fn method<'l>(
        &'l mut self,
        trigger_message: EnvMessage,
        dispatch: &'l mut Dispatch,
    ) -> ListenerMethodReturn {
        Box::pin(async {
            match trigger_message {
                EnvMessage::Request(EnvRequest::GetCompletion { ref agent_id, .. }) => {
                    if agent_id == InnerAgent::RagAssistant.id() {
                        let rag_assistant = dispatch
                            .get_agent_mut(&agent_id)
                            .map_err(|_| ListenerError::NoAgent)?;

                        if let Some(message) = self
                            .db_update
                            .write()
                            .expect("failed to write lock db update")
                            .take()
                        {
                            rag_assistant.cache.push(message)
                        }
                    }
                }
                EnvMessage::Response(EnvNotification::AgentStateUpdate {
                    ref agent_id,
                    ref cache,
                    ..
                }) => {
                    if agent_id == InnerAgent::QuickAssistant.id() {
                        let rag_assistant = dispatch
                            .get_agent_mut(&InnerAgent::RagAssistant.id())
                            .map_err(|_| ListenerError::NoAgent)?;

                        let assistant_role = MessageRole::Other {
                            alias: "QUICK_ASSISTANT".to_string(),
                            coerce_to: OtherRoleTo::Assistant,
                        };
                        let user_role = MessageRole::Other {
                            alias: "QUICK_USER".to_string(),
                            coerce_to: OtherRoleTo::User,
                        };
                        rag_assistant
                            .cache
                            .mut_filter_by(assistant_role.clone(), false);
                        rag_assistant.cache.mut_filter_by(user_role.clone(), false);

                        for mut message in cache.clone().into_iter() {
                            match message.role {
                                MessageRole::User => {
                                    message.role = user_role.clone();
                                }
                                MessageRole::Assistant => {
                                    message.role = assistant_role.clone();
                                }
                                _ => {}
                            }

                            rag_assistant.cache.push(message);
                        }
                    }
                }
                _ => {}
            }
            Ok(trigger_message)
        })
    }
    fn trigger<'l>(&self, env_message: &'l EnvMessage) -> Option<&'l EnvMessage> {
        match env_message {
            EnvMessage::Request(EnvRequest::GetCompletion { agent_id, .. }) => {
                if agent_id == InnerAgent::RagAssistant.id() {
                    if self.db_update.read().ok()?.is_none() {
                        error!("DB SHOULD NOT UPDATE AGENT");
                        return None;
                    }
                    return Some(env_message);
                }
            }
            EnvMessage::Response(EnvNotification::AgentStateUpdate {
                agent_id, cache, ..
            }) => {
                if agent_id == InnerAgent::QuickAssistant.id() {
                    return Some(env_message);
                }
            }
            _ => {}
        }

        None
    }
}
