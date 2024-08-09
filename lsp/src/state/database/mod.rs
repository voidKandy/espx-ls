pub mod error;
pub mod handle;
pub mod models;
use self::error::DatabaseResult;
use crate::config::{DatabaseConfig, GLOBAL_CONFIG};
use anyhow::anyhow;
use error::DatabaseError;
use handle::DatabaseHandle;
use serde::Deserialize;
use std::{path::PathBuf, time::Duration};
use surrealdb::{
    engine::{
        local::{Db, File},
        remote::ws::{Client, Ws},
    },
    opt::{auth::Root, Config},
    sql::{statements::RemoveDatabaseStatement, Thing},
    Surreal,
};
use tokio::time::sleep;
use tracing::{debug, info, warn};

#[derive(Debug)]
pub struct Database {
    // pub client: Surreal<Client>,
    pub client: Surreal<Db>,
    // config: DatabaseConfig,
    // path: PathBuf,
    handle: Option<DatabaseHandle>,
}

impl Drop for Database {
    fn drop(&mut self) {
        if let Some(h) = self.handle.take() {
            debug!("dropping database handle");
            h.kill().unwrap();
        }
    }
}

#[derive(Debug, Deserialize)]
pub struct Record {
    #[allow(dead_code)]
    id: Thing,
}

impl Database {
    // #[tracing::instrument(name = "connecting to database")]
    // async fn connect(
    //     config: &DatabaseConfig,
    // ) -> DatabaseResult<(Surreal<Client>, Option<DatabaseHandle>)> {
    //     let client: Surreal<Client> = Surreal::init();
    //     let url = format!("{}:{}", config.host, config.port);
    //     let mut handle = Option::<DatabaseHandle>::None;
    //
    //     match client.connect::<Ws>(&url).await {
    //         Ok(_) => {
    //             debug!("connected successfully, database handle is elsewhere");
    //             return Ok((client, handle));
    //         }
    //         Err(err) => {
    //             if let surrealdb::Error::Api(ref err) = err {
    //                 if let surrealdb::error::Api::Ws(err) = err {
    //                     // IO error: Connection refused (os error 61)
    //                     // ^^ the error when a database is uninitialized
    //                     if err.contains("refused") | err.contains("61") {
    //                         debug!("database is not running, starting database");
    //                         handle = Some(
    //                             DatabaseHandle::try_init(config)
    //                                 .ok_or(anyhow!("could not initialize the database handle"))?,
    //                         );
    //                         sleep(Duration::from_millis(300)).await;
    //                         debug!("reattempting connection");
    //                         client.connect::<Ws>(&url).await.map_err(|err| {
    //                             DatabaseError::Initialization(format!(
    //                                 "error occurred when reattempting to connect: {:?}",
    //                                 err
    //                             ))
    //                         })?;
    //                         return Ok((client, handle));
    //                     }
    //                 }
    //             }
    //             let msg = format!("failed connection to database: {:?}", err);
    //             debug!(msg);
    //             return Err(DatabaseError::Initialization(msg));
    //         }
    //     }
    // }

    #[tracing::instrument(name = "initialize database connection", skip_all)]
    pub async fn init(config: &DatabaseConfig) -> DatabaseResult<Self> {
        let path = GLOBAL_CONFIG.database_directory();
        debug!("path of database: {:?}", path);

        let root = Root {
            username: &config.user,
            password: &config.pass,
        };

        let cfg = Config::new().user(root);

        // let (client, handle) = Self::connect(config).await.expect("failed to connect");
        let client = Surreal::new::<File>((path, cfg)).await?;
        let handle = None;

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

        Ok(Self { client, handle })
    }

    // pub async fn import(&self) -> DatabaseResult<()> {
    // self.client
    //     .import(GLOBAL_CONFIG.database_directory())
    //     .await?;
    //     Ok(())
    // }

    // #[tracing::instrument(name = "exporting database", skip_all)]
    // pub async fn export(&self) -> DatabaseResult<()> {
    // self.client
    //     .export(GLOBAL_CONFIG.database_directory())
    //     .await?;
    //     Ok(())
    // }

    #[tracing::instrument(name = "importing database", skip_all)]
    pub async fn kill_handle(&mut self) -> DatabaseResult<()> {
        if let Some(h) = self.handle.take() {
            h.kill()?;
        }
        Ok(())
    }
}
