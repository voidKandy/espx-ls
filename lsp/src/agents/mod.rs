use std::collections::HashMap;

use espionox::{agents::Agent, prelude::Message};
use inits::doc_control_role;
use lsp_types::Uri;

use crate::config::espx::ModelConfig;

mod inits;

#[derive(Debug)]
pub struct Agents {
    pub config: ModelConfig,
    global: Agent,
    document: HashMap<Uri, Agent>,
}

impl From<ModelConfig> for Agents {
    fn from(cfg: ModelConfig) -> Self {
        let global = self::inits::global(&cfg);
        Self {
            config: cfg,
            global,
            document: HashMap::new(),
        }
    }
}

impl Agents {
    pub fn global_agent(&mut self) -> &mut Agent {
        &mut self.global
    }

    pub fn doc_agent(&mut self, uri: &Uri) -> Option<&mut Agent> {
        self.document.get_mut(uri)
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
}
