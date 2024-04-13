use std::{
    fmt::format,
    path::PathBuf,
    sync::{Arc, Mutex},
};

use espionox::{
    agents::memory::{Message, MessageRole},
    environment::{
        dispatch::{
            listeners::ListenerMethodReturn, Dispatch, EnvListener, EnvMessage, EnvNotification,
        },
        EnvError, ListenerError,
    },
};
use log::debug;

use crate::{burns::BufferBurn, config::GLOBAL_CONFIG, espx_env::agents::inner::InnerAgent};

// ONLY TRIGGERS FOR ASSISTANT INNER AGENT
#[derive(Debug)]
pub struct ConversationUpdate {
    pub conversation_file_path: PathBuf,
}

impl ConversationUpdate {
    pub fn init() -> Result<Self, EnvError> {
        let conversation_file_path = GLOBAL_CONFIG.paths.conversation_file_path.clone();
        Ok(Self {
            conversation_file_path,
        })
    }

    // For making the role look 𝐍 𝐈 𝐂 𝐄
    fn convert_ascii(str: &str, target: char) -> String {
        let start_code_point = target as u32;
        let str = str.to_lowercase();
        let mut chars = vec![' '];
        str.chars().for_each(|c| {
            let offset = c as u32 - 'a' as u32;
            chars.push(std::char::from_u32(start_code_point + offset).unwrap_or(c));
            chars.push(' ');
        });

        chars.into_iter().collect()
    }

    // For splitting the content of each message
    fn split_message(message: &str, chunk_size: usize) -> Vec<String> {
        message
            .chars()
            .collect::<Vec<char>>()
            .chunks(chunk_size)
            .map(|chunk| chunk.iter().collect())
            .collect()
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
                        debug!("CONVERSATION UPDATE ITERATION: {}", message);
                        let role_str = {
                            if let MessageRole::Other { alias, .. } = &message.role {
                                alias.to_string()
                            } else {
                                message.role.to_string()
                            }
                        };
                        let role_str = Self::convert_ascii(&role_str, '𝐀');
                        debug!("CONVERSATION UPDATE PUSHING: {}", role_str);
                        out_string_vec.push(format!("# {}\n\n", &role_str));

                        for chunk in Self::split_message(&message.content, 100) {
                            out_string_vec.push(chunk);
                            out_string_vec.push(String::from("\n"));
                        }
                    }
                    let content_to_write = out_string_vec.join("");
                    std::fs::write(self.conversation_file_path.clone(), content_to_write)
                        .map_err(|err| ListenerError::Undefined(err.into()))?;
                    debug!("CONVERSATION FILE WRITTEN");
                }
            }
            Ok(trigger_message)
        })
    }

    fn trigger<'l>(&self, env_message: &'l EnvMessage) -> Option<&'l EnvMessage> {
        if let EnvMessage::Response(noti) = env_message {
            if let EnvNotification::AgentStateUpdate { agent_id, .. } = noti {
                if agent_id == InnerAgent::Assistant.id() {
                    debug!("Conversation updater listener should trigger");
                    return Some(env_message);
                }
            }
        }
        None
    }
}

mod tests {
    use crate::espx_env::ConversationUpdate;

    #[test]
    fn convert_ascii_test() {
        let test = "test";
        let target = " 𝐓 𝐄 𝐒 𝐓 ";
        let converted = ConversationUpdate::convert_ascii(test, '𝐀');
        assert_eq!(converted, target);
    }
}
