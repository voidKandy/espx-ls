mod database;
use crate::{
    config::{Config, DatabaseConfig, ModelConfig, UserActionConfig},
    state::store::GlobalStore,
};
use std::sync::LazyLock;
mod parsing;
mod store;
use tracing::debug;

pub use crate::error::TRACING;

pub fn init_test_tracing() {
    LazyLock::force(&TRACING);
    debug!("test tracing initialized");
}

pub fn test_config() -> Config {
    let database = Some(DatabaseConfig {
        port: 8080,
        namespace: "test".to_owned(),
        database: "test".to_owned(),
        host: "0.0.0.0".to_owned(),
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

pub async fn test_store() -> GlobalStore {
    GlobalStore::from_config(&test_config()).await
}
