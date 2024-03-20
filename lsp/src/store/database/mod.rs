pub mod integrations;
pub mod models;

use lsp_types::Url;
use models::*;
use once_cell::sync::Lazy;
use serde::Deserialize;
use std::time::Duration;
use surrealdb::{
    engine::remote::ws::{Client, Ws},
    sql::Thing,
    Surreal,
};
use tokio::{
    process::{Child, Command},
    task::JoinHandle,
    time::sleep,
};

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

// GOOD FOR STATIC BUT NEED ALTERNATIVE
// pub async fn connect_db(namespace: &str, database: &str) {
//     sleep(Duration::from_millis(300)).await;
//     DB.client
//         .connect::<Ws>("0.0.0.0:8080")
//         .await
//         .expect("Failed to connect DB");
//     DB.client.use_ns(namespace).use_db(database).await.unwrap();
// }

impl Database {
    fn init() -> Self {
        let client = Surreal::init();
        let handle = DatabaseHandle::init();
        Self { client, handle }
    }

    pub async fn connect_db(&self, namespace: &str, database: &str) {
        sleep(Duration::from_millis(300)).await;
        self.client
            .connect::<Ws>("0.0.0.0:8080")
            .await
            .expect("Failed to connect DB");
        self.client
            .use_ns(namespace)
            .use_db(database)
            .await
            .unwrap();
    }

    pub async fn insert_document(&self, doc: &DBDocument) -> Result<Record, anyhow::Error> {
        let r = self
            .client
            .create((DBDocument::db_id(), doc.url.as_str()))
            .content(doc)
            .await?
            .expect("Failed to insert");
        Ok(r)
    }

    pub async fn insert_chunks(
        &self,
        chunks: &Vec<DBDocumentChunk>,
    ) -> Result<Vec<Record>, anyhow::Error> {
        let mut records = vec![];
        for chunk in chunks.iter() {
            records.append(
                &mut self
                    .client
                    .create(DBDocumentChunk::db_id())
                    .content(chunk)
                    .await?,
            )
        }
        Ok(records)
    }

    pub async fn remove_doc_by_url(&self, url: &Url) -> Result<Option<DBDocument>, anyhow::Error> {
        Ok(self
            .client
            .delete((DBDocument::db_id(), url.as_str()))
            .await
            .expect("Failed to delete"))
    }

    pub async fn remove_chunks_by_url(&self, url: &Url) -> Result<(), anyhow::Error> {
        let query = format!(
            "DELETE {} WHERE parent_url = $url",
            DBDocumentChunk::db_id()
        );

        self.client.query(query).bind(("url", url)).await?;
        Ok(())
    }

    pub async fn get_doc_by_url(&self, url: &Url) -> Result<Option<DBDocument>, anyhow::Error> {
        let query = format!(
            "SELECT * FROM ONLY {} where url = $url LIMIT 1",
            DBDocument::db_id()
        );

        let mut response = self.client.query(query).bind(("url", url)).await?;
        let doc: Option<DBDocument> = response.take(0)?;
        Ok(doc)
    }

    pub async fn get_chunks_by_url(
        &self,
        url: &Url,
    ) -> Result<Vec<DBDocumentChunk>, anyhow::Error> {
        let query = format!(
            "SELECT * FROM {} WHERE parent_url == $url",
            DBDocumentChunk::db_id()
        );

        let mut response = self.client.query(query).bind(("url", url)).await?;
        let docs: Vec<DBDocumentChunk> = response.take(0)?;
        Ok(docs)
    }

    pub async fn get_relavent_docs(
        &self,
        embedding: Vec<f32>,
        threshold: f32,
    ) -> Result<Vec<DBDocument>, anyhow::Error> {
        let query = format!("SELECT summary, url FROM documents WHERE vector::similarity::cosine(summary_embedding, $embedding) > {};", threshold );

        let mut response = self
            .client
            .query(query)
            .bind(("embedding", embedding))
            .await?;
        let docs: Vec<DBDocument> = response.take(0)?;
        Ok(docs)
    }
}

impl DatabaseHandle {
    fn init() -> Self {
        println!("RUNNING DATABASE INIT");
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
                // "file:espx-ls.db",
                "memory",
            ])
            .spawn()
            .expect("Failed to run database start command")
    }
}

mod tests {
    #[allow(unused)]
    use super::{DBDocument, DBDocumentChunk, Database, DatabaseHandle};
    #[allow(unused)]
    use lsp_types::Url;
    #[allow(unused)]
    use serde::{Deserialize, Serialize};
    #[allow(unused)]
    use std::time::Duration;
    #[allow(unused)]
    use surrealdb::engine::remote::ws::Ws;
    #[allow(unused)]
    use surrealdb::Surreal;
    #[allow(unused)]
    use tokio::time::sleep;

    fn test_doc_data() -> (DBDocument, Vec<DBDocumentChunk>) {
        let url = Url::parse("file:///tmp/foo").unwrap();
        let doc = DBDocument {
            url: url.clone(),
            summary: "This is a summary".to_owned(),
            summary_embedding: vec![0.1, 2.2, 3.4, 9.1, 0.3],
        };

        let chunks = vec![
            DBDocumentChunk {
                parent_url: url.clone(),
                summary: "This is chunk 1 summary".to_owned(),
                content: "This is chunk 1 content".to_owned(),
                summary_embedding: vec![1.1, 2.3, 92.0, 3.4, 3.3],
                content_embedding: vec![1.1, 2.3, 92.0, 3.4, 3.3],
                range: (0, 1),
            },
            DBDocumentChunk {
                parent_url: url.clone(),
                summary: "This is chunk 2 summary".to_owned(),
                content: "This is chunk 2 content".to_owned(),
                summary_embedding: vec![1.1, 2.3, 92.0, 3.4, 3.3],
                content_embedding: vec![1.1, 2.3, 92.0, 3.4, 3.3],
                range: (1, 2),
            },
            DBDocumentChunk {
                parent_url: url.clone(),
                summary: "This is chunk 3 summary".to_owned(),
                content: "This is chunk 3 content".to_owned(),
                summary_embedding: vec![1.1, 2.3, 92.0, 3.4, 3.3],
                content_embedding: vec![1.1, 2.3, 92.0, 3.4, 3.3],
                range: (2, 3),
            },
            DBDocumentChunk {
                parent_url: url.clone(),
                summary: "This is chunk 4 summary".to_owned(),
                content: "This is chunk 4 content".to_owned(),
                summary_embedding: vec![1.1, 2.3, 92.0, 3.4, 3.3],
                content_embedding: vec![1.1, 2.3, 92.0, 3.4, 3.3],
                range: (3, 4),
            },
            DBDocumentChunk {
                parent_url: url.clone(),
                summary: "This is chunk 5 summary".to_owned(),
                content: "This is chunk 5 content".to_owned(),
                summary_embedding: vec![1.1, 2.3, 92.0, 3.4, 3.3],
                content_embedding: vec![1.1, 2.3, 92.0, 3.4, 3.3],
                range: (5, 6),
            },
        ];
        (doc, chunks)
    }

    #[tokio::test]
    async fn database_spawn_crud_test() {
        let db = Database::init();
        sleep(Duration::from_millis(300)).await;
        db.connect_db("test", "test").await;
        let (doc, chunks) = test_doc_data();

        let rec = db.insert_document(&doc).await;
        println!("DOCUMENT RECORDS: {:?}", rec);

        let rec = db.insert_chunks(&chunks).await;
        println!("CHUNKS RECORDS: {:?}", rec);

        let got_chunks = db.get_chunks_by_url(&doc.url).await.unwrap();
        assert_eq!(chunks.len(), got_chunks.len());

        let got_doc = db.get_doc_by_url(&doc.url).await.unwrap();
        assert_eq!(doc.summary, got_doc.unwrap().summary);

        let _ = db.remove_doc_by_url(&doc.url).await;
        let _ = db.remove_chunks_by_url(&doc.url).await;

        let got_chunks = db.get_chunks_by_url(&doc.url).await.unwrap();
        assert_eq!(0, got_chunks.len());
        let got_doc = db.get_doc_by_url(&doc.url).await.unwrap();
        assert!(got_doc.is_none());
        db.handle.kill().await.unwrap();
    }
}
