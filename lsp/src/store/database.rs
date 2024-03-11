use std::collections::HashMap;

use crate::espx_env::{summarize, SUMMARIZE_WHOLE_DOC_PROMPT};

use super::{chunks::DocumentChunk, Document, Summarizable};
use espionox::agents::{language_models::embed, memory::embeddings::EmbeddingVector};
use lsp_types::Url;
use serde::{Deserialize, Serialize};
use tokio::process::{Child, Command};

#[derive(Clone, Serialize, Deserialize)]
#[serde(bound(deserialize = "'de: 'db"))]
pub struct DBDocument<'db> {
    url: Url,
    summary: &'db str,
    summary_embedding: Vec<f32>,
}

#[derive(Clone, Serialize, Deserialize)]
#[serde(bound(deserialize = "'de: 'db"))]
pub struct DBDocumentChunk<'db> {
    parent_url: Url,
    content: &'db str,
    content_embedding: Vec<f32>,
    summary: &'db str,
    summary_embedding: Vec<f32>,
    range: (usize, usize),
}

pub type DBDocumentTuple<'db> = (DBDocument<'db>, Vec<DBDocumentChunk<'db>>);

impl<'db> From<DBDocumentTuple<'db>> for Document {
    fn from((dbdoc, dbdoc_chunks): DBDocumentTuple<'db>) -> Self {
        let chunks = dbdoc_chunks
            .into_iter()
            .map(|ch| DocumentChunk {
                range: ch.range,
                content: ch.content.to_owned(),
                summary: Some(ch.summary.to_owned()),
                changes: HashMap::new(),
            })
            .collect();
        Document {
            url: dbdoc.url,
            chunks,
            summary: Some(dbdoc.summary.to_owned()),
        }
    }
}

async fn doc_as_db_tuple<'db>(
    doc: &'db mut Document,
) -> Result<DBDocumentTuple<'db>, anyhow::Error> {
    if doc.summary.is_none() {
        doc.get_summary().await?;
    }
    let summary = doc.summary.as_ref().unwrap();
    let summary_embedding = embed(&summary).unwrap();
    let dbdoc = DBDocument {
        url: doc.url.clone(),
        summary,
        summary_embedding,
    };

    let mut dbdoc_chunks = vec![];
    for ch in doc.chunks.iter_mut() {
        if ch.summary.is_none() {
            ch.get_summary().await?;
        }
        let summary = ch.summary.as_ref().unwrap();
        let summary_embedding = embed(&summary).unwrap();

        let content_embedding = embed(&ch.content).unwrap();

        let dbchunk = DBDocumentChunk {
            parent_url: doc.url.clone(),
            content: &ch.content,
            content_embedding,
            summary,
            summary_embedding,
            range: ch.range,
        };
        dbdoc_chunks.push(dbchunk);
    }
    Ok((dbdoc, dbdoc_chunks))
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
            "memory",
        ])
        .spawn()
        .expect("Failed to run database start command")
}

mod tests {
    use std::collections::HashMap;
    use std::time::Duration;

    use crate::espx_env::init_static_env_and_handle;
    use crate::store::chunks::DocumentChunk;
    use crate::store::Document;
    use espionox::agents::language_models::embed;
    use espionox::agents::memory::embeddings::EmbeddingVector;
    use lsp_types::{Documentation, Url};

    use super::doc_as_db_tuple;
    use super::start_database;
    use serde::{Deserialize, Serialize};
    use surrealdb::engine::local::Mem;
    use surrealdb::engine::remote::ws::Ws;
    use surrealdb::opt::auth::Root;
    use surrealdb::sql::Thing;
    use surrealdb::Surreal;
    use tokio::time::sleep;

    fn test_doc() -> Document {
        let chunk1 = DocumentChunk {
            range: (0, 2),
            changes: HashMap::new(),
            content: r#"This
            is 
            chunk1"#
                .to_string(),
            summary: Some("Summary of chunk1".to_string()),
        };

        let chunk2 = DocumentChunk {
            range: (3, 5),
            changes: HashMap::new(),
            content: r#"This
            is 
            chunk2"#
                .to_string(),
            summary: Some("This chunk mentions dogs".to_string()),
        };
        let chunk3 = DocumentChunk {
            range: (6, 8),
            changes: HashMap::new(),
            content: r#"This
            is 
            chunk3"#
                .to_string(),
            summary: Some("Summary of chunk3".to_string()),
        };
        let url = Url::parse("file:///tmp/foo").unwrap();

        Document {
            url,
            summary: Some("Summary of whole doc".to_string()),
            chunks: vec![chunk1, chunk2, chunk3],
        }
    }

    #[derive(Debug, Deserialize)]
    struct Record {
        #[allow(dead_code)]
        id: Thing,
    }

    #[tokio::test]
    async fn database() {
        init_static_env_and_handle().await;
        tokio::task::spawn(async { start_database() });
        println!("Spawned database thread");
        sleep(Duration::from_millis(200)).await;
        let db = Surreal::new::<Ws>("0.0.0.0:8080")
            .await
            .expect("failed to connect");
        db.use_ns("test").use_db("test").await.unwrap();
        let mut doc = test_doc();
        let url = doc.url.clone();
        let (dbdoc, dbdoc_chunks) = doc_as_db_tuple(&mut doc).await.unwrap();

        let created: Record = db
            .create(("document", url.as_str()))
            .content(dbdoc)
            .await
            .unwrap()
            .unwrap();
        println!("DOCUMENT RECORD CREATED: {:?}", created);

        for chunk in dbdoc_chunks.iter() {
            let rec: Vec<Record> = db.create("doc_chunk").content(chunk).await.unwrap();
            println!("Chunk record vec: {:?}", rec);
        }

        let embedding = embed("Dog facts").unwrap();
        let cosine_sql =
            "SELECT id, parent_url FROM doc_chunk WHERE summary_embedding <2,COSINE> $embedding;";

        let result = db
            .query(cosine_sql)
            .bind(("embedding", embedding))
            .await
            .unwrap();
        println!("COSINE: {:?}", result);

        let people: Vec<Record> = db.select("document").await.unwrap();
        println!("{:?}", people);
        assert!(false);
    }
}
