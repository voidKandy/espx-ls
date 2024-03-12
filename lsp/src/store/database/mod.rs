pub mod integrations;

use integrations::*;
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

pub static DB: Lazy<Database> = Lazy::new(Database::init);

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

    // pub async fn get_relavent_docs<'db>(
    //     &self,
    //     embedding: Vec<f32>,
    //     threshold: f32,
    // ) -> Result<Vec<DBDocument<'db>>, anyhow::Error> {
    // let query = format!("SELECT summary, url FROM documents WHERE vector::similarity::cosine(summary_embedding, $embedding) > {};", threshold );
    //
    // let mut response = self
    //     .client
    //     .query(query)
    //     .bind(("embedding", embedding))
    //     .await?;
    // let docs: Vec<DBDocument<'db>> = response.take(0)?;
    // Ok(docs)
    // }
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
    use super::DBDocument;
    use super::DatabaseHandle;
    use lsp_types::Url;
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
        let data = DBDocument {
            url: Url::parse("file:///tmp/foo").unwrap(),
            summary: "This is a summary".to_owned(),
            summary_embedding: vec![0.1, 2.2, 3.4, 9.1, 0.3],
        };
        let created: Vec<super::Record> = db.create("test_data").content(data).await.unwrap();
        println!("CREATED: {:?}", created);
        let query ="SELECT * FROM test_data WHERE vector::similarity::cosine(summary_embedding, $embedding) > 0.0;";
        let mut response = db
            .query(query)
            .bind(("embedding", [0.1, 2.2, 3.4, 9.1, 0.3]))
            .await
            .unwrap();
        println!("{:?}", response);
        let docs: Vec<DBDocument> = response.take(0).unwrap();

        // Delete test data
        let _: Vec<DBDocument> = db.delete("test_data").await.unwrap();

        assert_eq!(docs[0].summary, "This is a summary");
        db_thread.kill().await.unwrap();
    }
}
