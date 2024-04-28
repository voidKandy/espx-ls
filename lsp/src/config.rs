use espionox::language_models::ModelProvider;
use lsp_types::Url;
use once_cell::sync::Lazy;
use serde::{Deserialize, Serialize};
use std::{
    fs,
    path::{Path, PathBuf},
};
use toml;

pub static GLOBAL_CONFIG: Lazy<Box<Config>> = Lazy::new(|| Box::new(Config::default()));

#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct Config {
    pub user_actions: UserActionConfig,
    pub paths: EssentialPathsConfig,
    pub model: Option<ModelConfig>,
    pub database: Option<DatabaseConfig>,
}

#[derive(Debug, Deserialize, Serialize, Clone)]
pub struct DatabaseConfig {
    pub port: i32,
    pub namespace: String,
    pub database: String,
    pub host: Option<String>,
    pub user: Option<String>,
    pub pass: Option<String>,
}

#[derive(Debug, Deserialize, Serialize, Clone)]
pub struct ModelConfig {
    pub provider: ModelProvider,
    pub api_key: String,
}

#[derive(Debug, Deserialize, Serialize, Clone)]
pub struct UserActionConfig {
    pub io_trigger: String,
}

#[derive(Debug, Deserialize, Serialize, Clone)]
pub struct EssentialPathsConfig {
    pub conversation_file_path: PathBuf,
}

impl Default for Config {
    fn default() -> Self {
        let path = Path::new("espx-ls.toml");
        let mut pwd = std::env::current_dir().unwrap().canonicalize().unwrap();
        log::info!("PWD: {}", pwd.display());
        pwd.push(path);

        if let None = fs::read_dir(".espx-ls").ok() {
            fs::create_dir(".espx-ls").unwrap();
        }

        log::info!("CONFIG FILE PATH: {:?}", pwd);
        let content = fs::read_to_string(pwd).unwrap_or(String::new());
        log::info!("CONFIG FILE CONTENT: {:?}", content);
        let config: FromFileConfig = match toml::from_str(&content) {
            Ok(c) => c,
            Err(err) => panic!("CONFIG ERROR: {:?}", err),
        };
        log::info!("CONFIG: {:?}", config);
        config.into()
    }
}

#[derive(Debug, Deserialize, Serialize, Clone)]
pub struct FromFileConfig {
    pub model: Option<ModelConfig>,
    pub user_actions: Option<UserActionConfig>,
    pub paths: Option<EssentialPathsConfig>,
    pub database: Option<DatabaseConfig>,
}

impl Into<Config> for FromFileConfig {
    fn into(self) -> Config {
        let user_actions = self.user_actions.unwrap_or(UserActionConfig::default());
        let paths = self.paths.unwrap_or(EssentialPathsConfig::default());
        Config {
            user_actions,
            paths,
            model: self.model,
            database: self.database,
        }
    }
}
impl Default for EssentialPathsConfig {
    fn default() -> Self {
        let mut conversation_file_path = std::env::current_dir().unwrap().canonicalize().unwrap();
        conversation_file_path.push(PathBuf::from(".espx-ls/conversation.md"));
        Self {
            conversation_file_path,
        }
    }
}

impl EssentialPathsConfig {
    pub fn conversation_file_url(&self) -> anyhow::Result<Url> {
        let path = &GLOBAL_CONFIG.paths.conversation_file_path;
        fs::OpenOptions::new().create(true).write(true).open(path)?;
        let path_str = format!("file:///{}", path.display().to_string());
        let uri = Url::parse(&path_str)?;
        Ok(uri)
    }
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
