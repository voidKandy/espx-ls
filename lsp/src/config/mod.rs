pub mod database;
pub mod espx;
pub mod scopes;
use database::{DatabaseConfig, DatabaseConfigFromFile};
use espx::ModelConfig;
use scopes::{ScopeConfig, ScopeConfigFromFile, ScopeSettings};
use serde::{Deserialize, Serialize};
use std::{
    collections::HashMap,
    fs::{self},
    path::{Path, PathBuf},
};
use toml;
use tracing::debug;

#[derive(Debug, Clone, Deserialize, Serialize, PartialEq, Eq)]
pub struct Config {
    pub pwd: PathBuf,
    pub model: Option<ModelConfig>,
    pub database: Option<DatabaseConfig>,
    pub scopes: Option<ScopeConfig>,
}

#[derive(Debug, Clone, Deserialize, Serialize, PartialEq, Eq)]
pub struct ConfigFromFile {
    model: Option<ModelConfig>,
    database: Option<DatabaseConfigFromFile>,
    scopes: Option<ScopeConfigFromFile>,
}

impl From<(ConfigFromFile, PathBuf)> for Config {
    fn from((cfg, pwd): (ConfigFromFile, PathBuf)) -> Self {
        let scopes: Option<HashMap<char, ScopeSettings>> = {
            if cfg.scopes.is_none() || cfg.scopes.as_ref().is_some_and(|hm| hm.is_empty()) {
                None
            } else {
                let mut map = HashMap::new();

                for (char, settings) in cfg.scopes.unwrap() {
                    map.insert(char, settings.into());
                }
                Some(map)
            }
        };

        Config {
            pwd,
            model: cfg.model,
            database: cfg.database.and_then(|db| Some(db.into())),
            scopes,
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
