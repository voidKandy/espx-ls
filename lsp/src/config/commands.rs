use std::collections::HashMap;

use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct CommandsConfig {
    pub scopes: Vec<char>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct CommandsConfigFromFile {
    scopes: Option<Vec<char>>,
}

impl From<CommandsConfigFromFile> for CommandsConfig {
    fn from(value: CommandsConfigFromFile) -> Self {
        Self {
            scopes: value.scopes.unwrap_or(Vec::new()),
        }
    }
}
