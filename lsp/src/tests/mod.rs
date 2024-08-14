mod database;
mod parsing;
mod state;

pub use crate::error::TRACING;
use crate::{
    config::{Config, DatabaseConfig, ModelConfig, UserActionConfig},
    state::{database::Database, store::GlobalStore},
};
use std::sync::LazyLock;
use tracing::debug;

pub fn init_test_tracing() {
    LazyLock::force(&TRACING);
    debug!("test tracing initialized");
}

pub fn test_config() -> Config {
    let database = Some(DatabaseConfig {
        namespace: "test".to_owned(),
        database: "test".to_owned(),
        user: "root".to_owned(),
        pass: "root".to_owned(),
    });
    let pwd = std::env::current_dir()
        .expect("failed to get current dir")
        .canonicalize()
        .expect("failed to canonicalize pwd");
    Config {
        user_actions: UserActionConfig::default(),
        pwd,
        model: None,
        database,
    }
}

pub async fn test_db() -> Database {
    Database::init(&test_config().database.unwrap())
        .await
        .unwrap()
}
