use std::collections::HashSet;

use super::ENVIRONMENT;
use crate::store::database::{
    integrations::{DBDocument, DBDocumentChunk},
    DB,
};
use espionox::{
    agents::{
        independent::IndependentAgent,
        language_models::{embed, LanguageModel},
        memory::{Message, MessageRole},
        Agent,
    },
    environment::{
        dispatch::{
            listeners::ListenerMethodReturn, Dispatch, EnvListener, EnvMessage, EnvRequest,
        },
        EnvError, ListenerError,
    },
};

#[derive(Debug)]
pub struct DocStoreRAG {
    id_of_agent_to_update: String,
    store_agent: IndependentAgent,
    // sources: HashSet<RAGSource>,
}

impl DocStoreRAG {
    pub async fn new(id: &str) -> Result<Self, EnvError> {
        let env = ENVIRONMENT.get().unwrap().lock().unwrap();
        let a = Agent::new("", LanguageModel::default_gpt());
        let store_agent = env.make_agent_independent(a).await?;

        Ok(Self {
            store_agent,
            id_of_agent_to_update: id.to_owned(),
            // sources,
        })
    }
}

impl EnvListener for DocStoreRAG {
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
                let mut last_user_message: Message = agent
                    .cache
                    .pop(Some(MessageRole::User))
                    .expect("No last user message");

                let embedding = embed(&last_user_message.content)?;
            }

            //     self.store_agent.agent.cache.push(Message::new_system("
            //           You will be given a user prompt. Considering their question, give a general description of what might
            //           be contained in a code file that would be relavent to their question."
            //     ));
            //     self.store_agent
            //         .agent
            //         .cache
            //         .push(Message::new_user(&last_user_message.content));
            //     let response = self
            //         .store_agent
            //         .io_completion()
            //         .await
            //         .expect("Failed to get IO completion of store agent");
            //     let agent_response_emb = EmbeddingVector::from(embed(&response)?);
            //
            //     // NEED TO PUT A MUTEX TO GLOBAL STORE IN THIS STRUCT
            //     let docs_map = self
            //         .gstore
            //         .documents
            //         .get_by_proximity(agent_response_emb, 0.3);
            //
            //     last_user_message.content =
            //         Self::prepare_function_prompt(&last_user_message.content, docs_map);
            //     agent.cache.push(last_user_message);
            //     let new_message = return Ok(trigger_message);
            // }
            // Err(ListenerError::IncorrectTrigger)
            Ok(trigger_message)
        })
    }
    fn trigger<'l>(&self, env_message: &'l EnvMessage) -> Option<&'l EnvMessage> {
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
