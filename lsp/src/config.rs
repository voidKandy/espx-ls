use serde::{Deserialize, Serialize};
use std::{
    fs::{self, Permissions},
    path::{Path, PathBuf},
    str::FromStr,
    sync::LazyLock,
};
use toml;
use tracing::{debug, warn};

pub static GLOBAL_CONFIG: LazyLock<Box<Config>> = LazyLock::new(|| Box::new(Config::init()));

#[derive(Debug, Clone, Deserialize, Serialize, PartialEq, Eq)]
pub struct Config {
    pub user_actions: UserActionConfig,
    pub pwd: PathBuf,
    pub model: Option<ModelConfig>,
    pub database: Option<DatabaseConfig>,
}

#[derive(Debug, Clone, Deserialize, Serialize, PartialEq, Eq)]
pub struct DatabaseConfig {
    pub namespace: String,
    pub database: String,
    pub user: String,
    pub pass: String,
}

impl Default for DatabaseConfig {
    fn default() -> Self {
        Self {
            namespace: "namespace".to_string(),
            database: "database".to_string(),
            user: "root".to_string(),
            pass: "root".to_string(),
        }
    }
}

#[derive(Debug, Clone, Deserialize, Serialize, PartialEq, Eq)]
pub enum ModelProvider {
    OpenAi,
    Anthropic,
}

#[derive(Debug, Clone, Deserialize, Serialize, PartialEq, Eq)]
pub struct ModelConfig {
    pub provider: ModelProvider,
    pub api_key: String,
}

#[derive(Debug, Clone, Deserialize, Serialize, PartialEq, Eq)]
pub struct UserActionConfig {
    pub quick_prompt: String,
    pub quick_prompt_echo: String,
    pub rag_prompt: String,
    pub rag_prompt_echo: String,
    pub walk_project: String,
    pub walk_project_echo: String,
    pub lock_chunk_into_context: String,
    pub lock_doc_into_context: String,
    pub lock_doc_echo: String,
}

impl Default for UserActionConfig {
    fn default() -> Self {
        Self {
            quick_prompt: "##".to_string(),
            quick_prompt_echo: "⚑".to_string(),
            rag_prompt: "$$".to_string(),
            rag_prompt_echo: "⧗".to_string(),
            walk_project: "@@".to_string(),
            walk_project_echo: "⧉".to_string(),
            lock_chunk_into_context: "#---#".to_string(),
            lock_doc_into_context: "$---$".to_string(),
            lock_doc_echo: "🔒".to_string(),
        }
    }
}

#[derive(Debug, Clone, Deserialize, Serialize, PartialEq, Eq)]
struct ConfigFromFile {
    model: Option<ModelConfig>,
    user_actions: Option<UserActionConfig>,
    database: Option<DatabaseConfigFromFile>,
}

#[derive(Debug, Clone, Deserialize, Serialize, PartialEq, Eq)]
struct DatabaseConfigFromFile {
    // port: Option<i32>,
    namespace: Option<String>,
    database: Option<String>,
    // host: Option<String>,
    user: Option<String>,
    pass: Option<String>,
}

impl Into<DatabaseConfig> for DatabaseConfigFromFile {
    fn into(self) -> DatabaseConfig {
        DatabaseConfig {
            // port: self.port.unwrap_or_else(|| {
            //     let val = 5432;
            //     warn!("port not provided, defaulting to: {:?}", val);
            //     val
            // }),
            namespace: self.namespace.unwrap_or_else(|| {
                let val = "default_namespace";
                warn!("namespace not provided, defaulting to: {}", val);
                val.into()
            }),
            database: self.database.unwrap_or_else(|| {
                let val = "default_database";
                warn!("database not provided, defaulting to: {}", val);
                val.into()
            }),
            // host: self.host.unwrap_or_else(|| {
            //     let val = "0.0.0.0";
            //     warn!("host not provided, defaulting to: {}", val);
            //     val.into()
            // }),
            user: self.user.unwrap_or_else(|| {
                let val = "root";
                warn!("user not provided, defaulting to: {}", val);
                val.into()
            }),
            pass: self.pass.unwrap_or_else(|| {
                let val = "root";
                warn!("pass not provided, defaulting to: {}", val);
                val.into()
            }),
        }
    }
}

impl From<(ConfigFromFile, PathBuf)> for Config {
    fn from((cfg, pwd): (ConfigFromFile, PathBuf)) -> Self {
        let user_actions = cfg.user_actions.unwrap_or(UserActionConfig::default());
        Config {
            user_actions,
            pwd,
            model: cfg.model,
            database: cfg.database.and_then(|db| Some(db.into())),
        }
    }
}

impl Config {
    fn init() -> Self {
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
        "#;
        let pwd = PathBuf::from("~/Documents/projects/espx-ls/lsp");
        let expected = Config {
            user_actions: crate::config::UserActionConfig::default(),
            pwd: pwd.clone(),
            model: Some(ModelConfig {
                provider: crate::config::ModelProvider::Anthropic,
                api_key: "invalid".to_owned(),
            }),
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
