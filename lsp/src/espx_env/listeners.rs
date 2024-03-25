use espionox::{
    agents::{
        independent::IndependentAgent,
        memory::{Message, MessageRole},
    },
    environment::{
        dispatch::{
            listeners::ListenerMethodReturn, Dispatch, EnvListener, EnvMessage, EnvRequest,
        },
        EnvError, ListenerError,
    },
};

use super::agents::get_indy_agent;

#[derive(Debug)]
pub struct DocStoreRAG {
    id_of_agent_to_update: String,
    embedder: IndependentAgent,
}

impl DocStoreRAG {
    fn init(id_to_watch: &str) -> Result<Self, EnvError> {
        let embedder = get_indy_agent(super::agents::independent::IndyAgent::Embedder)
            .expect("Couldn't get indy agent")
            .clone();
        Ok(Self {
            embedder,
            id_of_agent_to_update: id_to_watch.to_owned(),
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
                let embedding = self
                    .embedder
                    .get_embedding(&last_user_message.content)
                    .await
                    .unwrap();
                // Add database querying logic
            }

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
