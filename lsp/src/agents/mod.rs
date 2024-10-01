use std::collections::HashMap;
pub mod error;
use crate::config::espx::ModelConfig;
use error::{AgentsError, AgentsResult};
use espionox::{
    agents::{memory::MessageStackRef, Agent},
    prelude::Message,
};
pub use inits::{doc_control_role, ASSISTANT_AGENT_SYSTEM_PROMPT};
use lsp_types::{MarkedString, Uri};
mod inits;

#[derive(Debug)]
pub struct Agents {
    pub config: ModelConfig,
    global: Agent,
    document: HashMap<Uri, Agent>,
    custom: HashMap<char, Agent>,
}

impl From<ModelConfig> for Agents {
    fn from(cfg: ModelConfig) -> Self {
        let global = self::inits::global(&cfg);
        Self {
            config: cfg,
            global,
            document: HashMap::new(),
            custom: HashMap::new(),
        }
    }
}

pub fn message_stack_into_marked_string(mut stack: MessageStackRef<'_>) -> MarkedString {
    let mut content = String::new();
    while let Some(message) = stack.pop(None) {
        content.push_str(&format!(
            r#"
## {:?}
{}
        "#,
            message.role,
            message.content.trim_start()
        ));
    }

    MarkedString::LanguageString(lsp_types::LanguageString {
        language: "markdown".to_string(),
        value: content,
    })
}

impl Agents {
    pub fn global_agent_ref(&self) -> &Agent {
        &self.global
    }

    pub fn global_agent_mut(&mut self) -> &mut Agent {
        &mut self.global
    }

    pub fn doc_agent_ref(&self, uri: &Uri) -> AgentsResult<&Agent> {
        self.document
            .get(uri)
            .ok_or(AgentsError::DocAgentNotPresent(uri.clone()))
    }

    pub fn doc_agent_mut(&mut self, uri: &Uri) -> AgentsResult<&mut Agent> {
        self.document
            .get_mut(uri)
            .ok_or(AgentsError::DocAgentNotPresent(uri.clone()))
    }

    pub fn custom_agent_mut(&mut self, char: char) -> AgentsResult<&mut Agent> {
        self.custom
            .get_mut(&char)
            .ok_or(AgentsError::CustomAgentNotPresent(char))
    }

    pub fn custom_agent_ref(&self, char: char) -> AgentsResult<&Agent> {
        self.custom
            .get(&char)
            .ok_or(AgentsError::CustomAgentNotPresent(char))
    }

    pub fn update_or_create_doc_agent(&mut self, uri: &Uri, doc_content: &str) {
        let role = doc_control_role();
        match self.document.get_mut(uri) {
            Some(agent) => {
                agent.cache.mut_filter_by(&role, false);
                agent.cache.push(Message {
                    role,
                    content: doc_content.to_owned(),
                });
            }
            None => {
                let agent = self::inits::document(&self.config, doc_content);
                self.document.insert(uri.clone(), agent);
            }
        }
    }

    pub fn create_custom_agent(&mut self, char: char, sys_prompt: String) {
        let agent = self::inits::custom(&self.config, sys_prompt);
        self.custom.insert(char, agent);
    }

    pub fn get_last_n_messages(agent: &Agent, n: usize) -> MessageStackRef {
        let messages: Vec<&Message> = agent.cache.as_ref().iter().rev().take(n).collect();
        MessageStackRef::from(messages)
    }
}
