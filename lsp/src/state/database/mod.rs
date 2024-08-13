pub mod error;
pub mod models;
use self::error::DatabaseResult;
use crate::config::{DatabaseConfig, GLOBAL_CONFIG};
use serde::Deserialize;
use surrealdb::{
    engine::local::{Db, File},
    opt::{auth::Root, Config},
    sql::Thing,
    Surreal,
};
use tracing::debug;

#[derive(Debug)]
pub struct Database {
    pub client: Surreal<Db>,
}

#[derive(Debug, Deserialize)]
pub struct Record {
    #[allow(dead_code)]
    id: Thing,
}

impl Database {
    #[tracing::instrument(name = "initialize database connection", skip_all)]
    pub async fn init(config: &DatabaseConfig) -> DatabaseResult<Self> {
        let path = GLOBAL_CONFIG.database_directory();
        debug!("path of database: {:?}", path);

        let root = Root {
            username: &config.user,
            password: &config.pass,
        };

        let cfg = Config::new().user(root);

        let client = Surreal::new::<File>((path, cfg)).await?;

        debug!("signing into database with credentials: {:?}", root);
        client.signin(root).await.expect("failed sign in");

        debug!(
            "namespace: {}\ndatabase: {}",
            config.namespace, config.database
        );
        client
            .use_ns(config.namespace.as_str())
            .use_db(config.database.as_str())
            .await
            .expect("failed to use database or namespace");

        Ok(Self { client })
    }
}
