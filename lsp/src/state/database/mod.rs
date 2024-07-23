pub mod error;
pub mod handle;
pub mod models;
use self::error::DatabaseResult;
use crate::config::DatabaseConfig;
use anyhow::anyhow;
use handle::DatabaseHandle;
use serde::Deserialize;
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
