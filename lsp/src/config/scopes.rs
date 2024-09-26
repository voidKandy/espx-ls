use std::collections::HashMap;

use serde::{Deserialize, Serialize};

use crate::agents::ASSISTANT_AGENT_SYSTEM_PROMPT;

pub type ScopeConfigFromFile = HashMap<char, ScopeSettingsFromFile>;
pub type ScopeConfig = HashMap<char, ScopeSettings>;

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct ScopeSettings {
    pub sys_prompt: String,
}

impl Default for ScopeSettings {
    fn default() -> Self {
        let sys_prompt = ASSISTANT_AGENT_SYSTEM_PROMPT.to_string();
        Self { sys_prompt }
    }
}

impl From<ScopeSettingsFromFile> for ScopeSettings {
    fn from(value: ScopeSettingsFromFile) -> Self {
        Self {
            sys_prompt: value.sys_prompt.unwrap_or(Self::default().sys_prompt),
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct ScopeSettingsFromFile {
    pub sys_prompt: Option<String>,
}
