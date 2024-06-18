use lsp_types::Uri;
use once_cell::sync::Lazy;
use serde::{Deserialize, Serialize};
use std::{
    fs,
    path::{Path, PathBuf},
    str::FromStr,
};
use toml;
use tracing::info;

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
pub enum ModelProvider {
    OpenAi,
    Anthropic,
}

#[derive(Debug, Deserialize, Serialize, Clone)]
pub struct ModelConfig {
    pub provider: ModelProvider,
    pub api_key: String,
}

#[derive(Debug, Deserialize, Serialize, Clone)]
pub struct UserActionConfig {
    pub quick_prompt: String,
    pub rag_prompt: String,
    pub walk_project: String,
}

#[derive(Debug, Deserialize, Serialize, Clone)]
pub struct EssentialPathsConfig {
    pub conversation_file_path: PathBuf,
}

impl Default for Config {
    fn default() -> Self {
        let path = Path::new("espx-ls.toml");
        let mut pwd = std::env::current_dir().unwrap().canonicalize().unwrap();
        info!("PWD: {}", pwd.display());
        pwd.push(path);

        if let None = fs::read_dir(".espx-ls").ok() {
            fs::create_dir(".espx-ls").unwrap();
        }

        info!("CONFIG FILE PATH: {:?}", pwd);
        let content = fs::read_to_string(pwd).unwrap_or(String::new());
        info!("CONFIG FILE CONTENT: {:?}", content);
        let config: FromFileConfig = match toml::from_str(&content) {
            Ok(c) => c,
            Err(err) => panic!("CONFIG ERROR: {:?}", err),
        };
        info!("CONFIG: {:?}", config);
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
    pub fn conversation_file_uri(&self) -> anyhow::Result<Uri> {
        let path = &GLOBAL_CONFIG.paths.conversation_file_path;
        fs::OpenOptions::new().create(true).write(true).open(path)?;
        let path_str = format!("file:///{}", path.display().to_string());
        let uri = Uri::from_str(&path_str)?;
        Ok(uri)
    }
}

impl Default for UserActionConfig {
    fn default() -> Self {
        Self {
            quick_prompt: "#$".to_string(),
            rag_prompt: "#$#".to_string(),
            walk_project: "@@".to_string(),
        }
    }
}

// Unused??
// I don't know why this is implemented
impl<'ac> Into<Vec<&'ac str>> for &'ac UserActionConfig {
    fn into(self) -> Vec<&'ac str> {
        vec![self.quick_prompt.as_str()]
    }
}
