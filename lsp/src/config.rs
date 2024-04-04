use espionox::language_models::ModelProvider;
use once_cell::sync::Lazy;
use serde::{Deserialize, Serialize};
use std::{fs, path::Path};
use toml;

pub static GLOBAL_CONFIG: Lazy<Box<Config>> = Lazy::new(|| Box::new(Config::get()));

#[derive(Debug, Deserialize, Serialize)]
pub struct Config {
    pub model: ModelConfig,
    pub user_actions: UserActionConfig,
}

#[derive(Debug, Deserialize, Serialize)]
pub struct FromFileConfig {
    pub model: ModelConfig,
    pub user_actions: Option<UserActionConfig>,
}

impl Into<Config> for FromFileConfig {
    fn into(self) -> Config {
        let user_actions = self.user_actions.unwrap_or(UserActionConfig::default());
        Config {
            model: self.model,
            user_actions,
        }
    }
}

#[derive(Debug, Deserialize, Serialize)]
pub struct ModelConfig {
    pub provider: ModelProvider,
    pub api_key: String,
}

#[derive(Debug, Deserialize, Serialize)]
pub struct UserActionConfig {
    pub io_trigger: String,
}

impl Default for UserActionConfig {
    fn default() -> Self {
        Self {
            io_trigger: "#$".to_string(),
        }
    }
}

impl<'ac> Into<Vec<&'ac str>> for &'ac UserActionConfig {
    fn into(self) -> Vec<&'ac str> {
        vec![self.io_trigger.as_str()]
    }
}

impl Config {
    pub fn get() -> Config {
        let path = Path::new("espx-ls.toml");
        let mut pwd = std::env::current_dir().unwrap().canonicalize().unwrap();
        log::info!("PWD: {}", pwd.display());
        pwd.push(path);

        log::info!("CONFIG FILE PATH: {:?}", pwd);
        let content = fs::read_to_string(pwd).unwrap();
        log::info!("CONFIG FILE CONTENT: {:?}", content);
        let config: FromFileConfig = match toml::from_str(&content) {
            Ok(c) => c,
            Err(err) => panic!("CONFIG ERROR: {:?}", err),
        };
        log::info!("CONFIG: {:?}", config);
        config.into()
    }
}
