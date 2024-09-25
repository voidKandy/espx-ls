pub mod commands;
pub mod database;
pub mod espx;
use commands::{CommandsConfig, CommandsConfigFromFile};
use database::{DatabaseConfig, DatabaseConfigFromFile};
use espx::ModelConfig;
use serde::{Deserialize, Serialize};
use std::{
    fs::{self},
    path::{Path, PathBuf},
    sync::LazyLock,
};
use toml;
use tracing::debug;

// pub static GLOBAL_CONFIG: LazyLock<Box<Config>> = LazyLock::new(|| Box::new(Config::init()));

#[derive(Debug, Clone, Deserialize, Serialize, PartialEq, Eq)]
pub struct Config {
    pub pwd: PathBuf,
    pub model: Option<ModelConfig>,
    pub database: Option<DatabaseConfig>,
    pub commands: Option<CommandsConfig>,
}

#[derive(Debug, Clone, Deserialize, Serialize, PartialEq, Eq)]
struct ConfigFromFile {
    model: Option<ModelConfig>,
    database: Option<DatabaseConfigFromFile>,
    commands: Option<CommandsConfigFromFile>,
}

impl From<(ConfigFromFile, PathBuf)> for Config {
    fn from((cfg, pwd): (ConfigFromFile, PathBuf)) -> Self {
        Config {
            pwd,
            model: cfg.model,
            database: cfg.database.and_then(|db| Some(db.into())),
            commands: cfg.commands.and_then(|com| Some(com.into())),
        }
    }
}

impl Config {
    pub fn init() -> Self {
        let pwd = std::env::current_dir()
            .expect("failed to get current dir")
            .canonicalize()
            .expect("failed to canonicalize pwd");
        debug!("pwd: {:?}", pwd);
        let mut config_file_path = pwd.clone();
        config_file_path.push(Path::new("espx-ls.toml"));

        let content = fs::read_to_string(config_file_path).unwrap_or(String::new());
        let cnfg: ConfigFromFile = match toml::from_str(&content) {
            Ok(c) => c,
            Err(err) => panic!("CONFIG ERROR: {:?}", err),
        };
        Config::from((cnfg, pwd))
    }

    fn espx_ls_dir(&self) -> PathBuf {
        let mut path = self.pwd.clone();
        path.push(PathBuf::from(".espx-ls"));
        debug!("espx ls folder path: {:?}", path);
        if !path.exists() {
            fs::create_dir(&path).expect("failed to make .espx-ls directory");
        }
        path
    }

    pub fn conversation_file(&self) -> PathBuf {
        let mut path = self.espx_ls_dir();
        path.push(PathBuf::from("conversation.md"));
        if !path.exists() {
            fs::File::create_new(&path).expect("failed to create conversation file");
        }
        path
    }

    pub fn database_directory(&self) -> PathBuf {
        let mut path = self.espx_ls_dir();
        path.push(PathBuf::from("db.surql"));
        path
    }
}

mod tests {
    use super::*;
    use espx::ModelProvider;
    use std::collections::HashMap;

    #[test]
    fn config_builds_correctly() {
        let input = r#"
            [model]
            provider="Anthropic"
            api_key="invalid"

            [database]
            namespace="espx" 
            database="espx"
            user="root"
            pass="root"

            [commands]
            scopes = [ "c" ]
        "#;
        let pwd = PathBuf::from("~/Documents/projects/espx-ls/lsp");
        let expected = Config {
            pwd: pwd.clone(),
            model: Some(ModelConfig {
                provider: ModelProvider::Anthropic,
                api_key: "invalid".to_owned(),
            }),
            commands: Some(CommandsConfig { scopes: vec!['c'] }),
            database: Some(crate::config::DatabaseConfig {
                namespace: "espx".to_owned(),
                database: "espx".to_owned(),
                user: "root".to_owned(),
                pass: "root".to_owned(),
            }),
        };

        let cnfg: ConfigFromFile = match toml::from_str(&input) {
            Ok(c) => c,
            Err(err) => panic!("CONFIG ERROR: {:?}", err),
        };

        debug!("got from file config: {:?}", cnfg);
        let cfg = Config::from((cnfg, pwd));

        assert_eq!(expected, cfg);
    }
}
