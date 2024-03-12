pub mod integrations;

use once_cell::sync::Lazy;
use serde::Deserialize;
use std::time::Duration;
use surrealdb::engine::remote::ws::Client;
use surrealdb::engine::remote::ws::Ws;
use surrealdb::sql::Thing;
use surrealdb::Surreal;
use tokio::process::{Child, Command};
use tokio::task::JoinHandle;
use tokio::time::sleep;

static DB: Lazy<Database> = Lazy::new(Database::init);

pub struct Database {
    pub client: Surreal<Client>,
    handle: DatabaseHandle,
}

struct DatabaseHandle(JoinHandle<Child>);

#[derive(Debug, Deserialize)]
struct Record {
    #[allow(dead_code)]
    id: Thing,
}

pub async fn connect_db(namespace: &str, database: &str) {
    sleep(Duration::from_millis(300)).await;
    DB.client
        .connect::<Ws>("0.0.0.0:8080")
        .await
        .expect("Failed to connect DB");
    DB.client.use_ns(namespace).use_db(database).await.unwrap();
}

impl Database {
    fn init() -> Self {
        let client = Surreal::init();
        let handle = DatabaseHandle::init();
        Self { client, handle }
    }
}

impl DatabaseHandle {
    fn init() -> Self {
        let handle = tokio::task::spawn(async { Self::start_database() });
        Self(handle)
    }
    async fn kill(self) -> Result<(), std::io::Error> {
        self.0.await.unwrap().kill().await?;
        Ok(())
    }
    fn start_database() -> Child {
        Command::new("surreal")
            .args([
                "start",
                "--log",
                "trace",
                "--user",
                "root",
                "--pass",
                "root",
                "--bind",
                "0.0.0.0:8080",
                "file:espx-ls.db",
                // "memory",
            ])
            .spawn()
            .expect("Failed to run database start command")
    }
}

mod tests {
    use super::DatabaseHandle;
    use serde::{Deserialize, Serialize};
    use std::time::Duration;
    use surrealdb::engine::remote::ws::Ws;
    use surrealdb::Surreal;
    use tokio::time::sleep;

    #[derive(Debug, Deserialize, Serialize)]
    struct TestData {
        content: String,
        number: i32,
    }

    #[tokio::test]
    async fn database_spawn_connect_kill() {
        let db_thread = DatabaseHandle::init();
        sleep(Duration::from_millis(300)).await;

        let db = Surreal::new::<Ws>("0.0.0.0:8080")
            .await
            .expect("failed to connect");
        db.use_ns("test").use_db("test").await.unwrap();
        let data = TestData {
            content: "CONTENT".to_owned(),
            number: 2,
        };
        let created: Vec<super::Record> = db.create("test_data").content(data).await.unwrap();
        let selected: Vec<super::Record> = db.select("test_data").await.unwrap();
        let d: Option<TestData> = db
            .delete(("test_data", selected[0].id.clone()))
            .await
            .unwrap();
        assert_eq!(d.unwrap().number, 2);
        db_thread.kill().await.unwrap();
    }
}
