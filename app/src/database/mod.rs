pub mod error;
pub mod models;
// pub mod vector_search;
use self::error::DatabaseResult;
use crate::config::{database::DatabaseConfig, Config};
use error::DatabaseError;
use serde::Deserialize;
use surrealdb::{
    engine::local::{Db, File, RocksDb},
    opt::{auth::Root, Config as SurConfig},
    sql::Thing,
    Surreal,
};
use tracing::debug;

#[derive(Debug)]
pub struct Database {
    pub config: DatabaseConfig,
    pub client: Surreal<Db>,
}

#[derive(Debug, Deserialize)]
pub struct Record {
    #[allow(dead_code)]
    id: Thing,
}

impl Database {
    #[tracing::instrument(name = "initialize database connection", skip_all)]
    pub async fn init(config: &mut Config) -> DatabaseResult<Self> {
        let path = config.database_directory();
        debug!("path of database: {:?}", path);

        let config = config
            .database
            .take()
            .ok_or(DatabaseError::Initialization(String::from(
                "No Configuration",
            )))?;

        let root = Root {
            username: &config.user,
            password: &config.pass,
        };

        let cfg = SurConfig::new().user(root);

        let client = Surreal::new::<RocksDb>((path, cfg)).await?;

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

        Ok(Self { client, config })
    }
}
