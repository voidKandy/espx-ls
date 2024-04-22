pub mod chunks;
pub mod docs;
pub mod error;
use self::error::DbModelResult;
use crate::config::DatabaseConfig;
use anyhow::anyhow;
use chunks::*;
use docs::*;
use log::info;
use lsp_types::Url;
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

#[derive(Debug)]
pub struct Database {
    pub client: Surreal<Client>,
    handle: Option<DatabaseHandle>,
}

#[derive(Debug)]
struct DatabaseHandle(JoinHandle<Child>);

#[derive(Debug, Deserialize)]
pub struct Record {
    #[allow(dead_code)]
    id: Thing,
}

impl Database {
    pub async fn init(config: &DatabaseConfig) -> DbModelResult<Self> {
        let client: Surreal<Client> = Surreal::init();
        let handle = Some(DatabaseHandle::init(config));

        info!("DB CLIENT AND HANDLE INITIATED, SLEEPING 300MS");
        sleep(Duration::from_millis(300)).await;

        let url = match &config.host {
            Some(host) => format!("{}:{}", host, config.port),
            None => format!("0.0.0.0:{}", config.port),
        };

        client.connect::<Ws>(url).await?;
        client
            .use_ns(config.namespace.as_str())
            .use_db(config.database.as_str())
            .await?;
        info!("DB CLIENT CONNECTED");

        Ok(Self { client, handle })
    }

    pub async fn kill_handle(&mut self) -> DbModelResult<()> {
        self.handle
            .take()
            .ok_or(anyhow!("Handle was none"))?
            .kill()
            .await?;
        Ok(())
    }

    // pub async fn connect_db(&self, namespace: &str, database: &str) {
    //     sleep(Duration::from_millis(300)).await;
    //     self.client
    //         .connect::<Ws>("0.0.0.0:8080")
    //         .await
    //         .expect("Failed to connect DB");
    //     self.client
    //         .use_ns(namespace)
    //         .use_db(database)
    //         .await
    //         .unwrap();
    // }

    pub async fn update_doc_store(&self, text: &str, url: &Url) -> DbModelResult<()> {
        info!("DID OPEN GOT DATABASE READ");
        match self.get_doc_tuple_by_url(&url).await? {
            None => {
                info!("DID OPEN NEEDS TO BUILD DB TUPLE");
                let tup = DBDocument::build_tuple(text.to_owned(), url.clone())
                    .await
                    .expect("Failed to build dbdoc tuple");
                info!("DID OPEN BUILT TUPLE");
                self.insert_document(&tup.0).await?;
                self.insert_chunks(&tup.1).await?;
            }
            Some((_, chunks)) => {
                info!("DID OPEN HAS TUPLE");
                if chunk_vec_content(&chunks) != text {
                    info!("DID OPEN UPDATING");
                    // THIS IS NOT A GOOD SOLUTION BECAUSE AT SOME POINT THE SUMMARY OF THE DOC
                    // ENTRY WILL DEPRECATE
                    // ALSO
                    // A PATCH WOULD BE BETTER THAN JUST DELETING AND REPLACING ALL OF THE CHUNKS
                    self.remove_chunks_by_url(&url)
                        .await
                        .expect("Could not remove chunks");
                    let chunks = DBDocumentChunk::chunks_from_text(url.clone(), &text).await?;
                    self.insert_chunks(&chunks).await?;
                }
            }
        }
        Ok(())
    }

    pub async fn get_doc_tuple_by_url(&self, url: &Url) -> DbModelResult<Option<DBDocumentTuple>> {
        info!("DB GETTING DOC TUPLE");
        let doc_opt = self.get_doc_by_url(url).await?;
        if let Some(doc) = doc_opt {
            let chunks = self.get_chunks_by_url(url).await?;
            return Ok(Some((doc, chunks)));
        }
        return Ok(None);
    }

    pub async fn insert_document(&self, doc: &DBDocument) -> DbModelResult<Record> {
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

    pub async fn remove_doc_by_url(&self, url: &Url) -> DbModelResult<Option<DBDocument>> {
        Ok(self
            .client
            .delete((DBDocument::db_id(), url.as_str()))
            .await
            .expect("Failed to delete"))
    }

    pub async fn remove_chunks_by_url(&self, url: &Url) -> DbModelResult<()> {
        let query = format!(
            "DELETE {} WHERE parent_url = $url",
            DBDocumentChunk::db_id()
        );

        self.client.query(query).bind(("url", url)).await?;
        Ok(())
    }

    pub async fn get_doc_by_url(&self, url: &Url) -> DbModelResult<Option<DBDocument>> {
        let query = format!(
            "SELECT * FROM ONLY {} where url = $url LIMIT 1",
            DBDocument::db_id()
        );

        info!("DB QUERY CONSTRUCTED");

        let mut response = self.client.query(query).bind(("url", url)).await?;
        info!("DB QUERY RESPONSE: {:?}", response);
        let doc: Option<DBDocument> = response.take(0)?;
        Ok(doc)
    }

    pub async fn get_chunks_by_url(&self, url: &Url) -> DbModelResult<ChunkVector> {
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
    ) -> DbModelResult<Vec<DBDocument>> {
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
    fn init(config: &DatabaseConfig) -> Self {
        let (user, pass, host, port) = (
            config.user.to_owned().unwrap_or("root".to_owned()),
            config.pass.to_owned().unwrap_or("root".to_owned()),
            config.host.to_owned().unwrap_or("0.0.0.0".to_owned()),
            config.port,
        );

        let handle =
            tokio::task::spawn(async move { Self::start_database(user, pass, host, port) });
        Self(handle)
    }

    async fn kill(self) -> Result<(), std::io::Error> {
        self.0.await.unwrap().kill().await?;
        Ok(())
    }

    fn start_database(user: String, pass: String, host: String, port: i32) -> Child {
        info!("DATABASE INITIALIZING");
        Command::new("surreal")
            .args([
                "start",
                "--log",
                "trace",
                "--user",
                &user,
                "--pass",
                &pass,
                "--bind",
                &format!("{}:{}", host, port),
            ])
            .spawn()
            .expect("Failed to run database start command")
    }
}

mod tests {
    use crate::config::DatabaseConfig;

    #[allow(unused)]
    use super::{DBDocument, DBDocumentChunk, Database, DatabaseHandle};
    #[allow(unused)]
    use lsp_types::Url;
    #[allow(unused)]
    use serde::{Deserialize, Serialize};
    use std::collections::HashMap;
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
            burns: HashMap::new(),
        };

        let chunks = vec![
            DBDocumentChunk {
                parent_url: url.clone(),
                // summary: "This is chunk 1 summary".to_owned(),
                // summary_embedding: vec![1.1, 2.3, 92.0, 3.4, 3.3],
                content: "This is chunk 1 content".to_owned(),
                content_embedding: vec![1.1, 2.3, 92.0, 3.4, 3.3],
                range: (0, 1),
            },
            DBDocumentChunk {
                parent_url: url.clone(),
                // summary: "This is chunk 2 summary".to_owned(),
                // summary_embedding: vec![1.1, 2.3, 92.0, 3.4, 3.3],
                content: "This is chunk 2 content".to_owned(),
                content_embedding: vec![1.1, 2.3, 92.0, 3.4, 3.3],
                range: (1, 2),
            },
            DBDocumentChunk {
                parent_url: url.clone(),
                // summary: "This is chunk 3 summary".to_owned(),
                // summary_embedding: vec![1.1, 2.3, 92.0, 3.4, 3.3],
                content: "This is chunk 3 content".to_owned(),
                content_embedding: vec![1.1, 2.3, 92.0, 3.4, 3.3],
                range: (2, 3),
            },
            DBDocumentChunk {
                parent_url: url.clone(),
                // summary: "This is chunk 4 summary".to_owned(),
                // summary_embedding: vec![1.1, 2.3, 92.0, 3.4, 3.3],
                content: "This is chunk 4 content".to_owned(),
                content_embedding: vec![1.1, 2.3, 92.0, 3.4, 3.3],
                range: (3, 4),
            },
            DBDocumentChunk {
                parent_url: url.clone(),
                // summary: "This is chunk 5 summary".to_owned(),
                // summary_embedding: vec![1.1, 2.3, 92.0, 3.4, 3.3],
                content: "This is chunk 5 content".to_owned(),
                content_embedding: vec![1.1, 2.3, 92.0, 3.4, 3.3],
                range: (5, 6),
            },
        ];
        (doc, chunks)
    }

    #[tokio::test]
    async fn database_spawn_crud_test() {
        let test_conf = DatabaseConfig {
            port: 8080,
            namespace: "test".to_owned(),
            database: "test".to_owned(),
            host: None,
            user: None,
            pass: None,
        };
        let mut db = Database::init(&test_conf)
            .await
            .expect("Failed to init database");
        sleep(Duration::from_millis(300)).await;
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
        assert!(db.kill_handle().await.is_ok());
    }
}
