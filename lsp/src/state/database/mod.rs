pub mod error;
pub mod handle;
pub mod models;
pub mod tests;
use self::error::DatabaseResult;
use crate::config::DatabaseConfig;
use anyhow::anyhow;
use handle::DatabaseHandle;
use lsp_types::Uri;
use models::{chunks::DBDocumentChunk, info::DBDocumentInfo, FullDBDocument};
use serde::{Deserialize, Serialize};
use std::time::Duration;
use surrealdb::{
    engine::remote::ws::{Client, Ws},
    sql::Thing,
    Surreal,
};
use tokio::time::sleep;
use tracing::info;

#[derive(Debug)]
pub struct Database {
    pub client: Surreal<Client>,
    handle: Option<DatabaseHandle>,
}

#[derive(Debug, Deserialize)]
pub struct Record {
    #[allow(dead_code)]
    id: Thing,
}
/// Anything that is inserted into the database should implement this trait
pub trait DatabaseStruct<R>: Serialize + for<'de> Deserialize<'de> + Sized {
    async fn insert(db: &Database, me: Self) -> DatabaseResult<Record> {
        let mut ret = db.client.create(Self::db_id()).content(me).await?;
        let r: Record = ret.remove(0);
        Ok(r)
    }
    fn db_id() -> &'static str;
    async fn get_all(db: &Database) -> DatabaseResult<Vec<Self>> {
        let query = format!("SELECT * FROM {}", Self::db_id());
        let mut response = db.client.query(query).await?;
        let r = response.take(0)?;
        Ok(r)
    }
    async fn take_all(db: &Database) -> DatabaseResult<Vec<Self>> {
        let query = format!("DELETE * FROM {}", Self::db_id());
        let mut response = db.client.query(query).await?;
        let r = response.take(0)?;
        Ok(r)
    }
    async fn get_all_by_uri(db: &Database, uri: &Uri) -> DatabaseResult<R>;
    async fn take_all_by_uri(db: &Database, uri: &Uri) -> DatabaseResult<R>;
}

impl Database {
    pub async fn init(config: &DatabaseConfig) -> DatabaseResult<Self> {
        let client: Surreal<Client> = Surreal::init();
        let handle = DatabaseHandle::try_init(config);

        info!("DB CLIENT AND HANDLE INITIATED, SLEEPING 300MS");
        sleep(Duration::from_millis(300)).await;

        let uri = match &config.host {
            Some(host) => format!("{}:{}", host, config.port),
            None => format!("0.0.0.0:{}", config.port),
        };
        info!("DB CONNECTION uri: {}", uri);

        client.connect::<Ws>(uri).await?;
        client
            .use_ns(config.namespace.as_str())
            .use_db(config.database.as_str())
            .await?;
        info!("DB CLIENT CONNECTED");

        Ok(Self { client, handle })
    }

    pub async fn kill_handle(&mut self) -> DatabaseResult<()> {
        self.handle
            .take()
            .ok_or(anyhow!("Handle was none"))?
            .kill()?;
        Ok(())
    }
}
