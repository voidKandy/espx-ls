use std::path::PathBuf;

use espionox::environment::{
    dispatch::{
        listeners::ListenerMethodReturn, Dispatch, EnvListener, EnvMessage, EnvNotification,
    },
    ListenerError,
};

use crate::espx_env::agents::inner::InnerAgent;

#[derive(Debug)]
pub struct ConversationUpdate {
    conversation_file_path: PathBuf,
}
// ONLY TRIGGERS FOR ASSISTANT INNER AGENT

impl Default for ConversationUpdate {
    fn default() -> Self {
        let mut conversation_file_path = std::env::current_dir().unwrap().canonicalize().unwrap();
        conversation_file_path.push(PathBuf::from(".espx-ls/conversation.md"));
        Self {
            conversation_file_path,
        }
    }
}

impl EnvListener for ConversationUpdate {
    fn method<'l>(
        &'l mut self,
        trigger_message: EnvMessage,
        _dispatch: &'l mut Dispatch,
    ) -> ListenerMethodReturn {
        Box::pin(async {
            if let EnvMessage::Response(ref noti) = trigger_message {
                if let EnvNotification::AgentStateUpdate { cache, .. } = noti {
                    let mut out_string_vec = vec![];
                    for message in cache.as_ref().into_iter() {
                        out_string_vec.push(format!("# {}\n", message.role.to_string()));
                        out_string_vec.push(format!("{}\n", message.content));
                    }

                    let content_to_write = out_string_vec.join("\n");
                    std::fs::write(self.conversation_file_path.clone(), content_to_write)
                        .map_err(|err| ListenerError::Undefined(err.into()))?;
                }
            }
            Ok(trigger_message)
        })
    }

    fn trigger<'l>(&self, env_message: &'l EnvMessage) -> Option<&'l EnvMessage> {
        if let EnvMessage::Response(noti) = env_message {
            if let EnvNotification::AgentStateUpdate { agent_id, .. } = noti {
                if agent_id == InnerAgent::Assistant.id() {
                    return Some(env_message);
                }
            }
        }
        None
    }
}
