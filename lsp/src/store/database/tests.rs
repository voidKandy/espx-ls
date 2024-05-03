#![allow(unused)]

use super::{DBDocument, DBDocumentChunk, Database};
use crate::config::DatabaseConfig;
use std::collections::HashMap;

use lsp_types::Url;
use serde::{Deserialize, Serialize};
use std::time::Duration;
use surrealdb::engine::remote::ws::Ws;
use surrealdb::Surreal;
use tokio::time::sleep;

fn test_doc_data() -> (DBDocument, Vec<DBDocumentChunk>) {
    let url = Url::parse("file:///tmp/foo").unwrap();
    let doc = DBDocument {
        url: url.clone(),
        // summary: "This is a summary".to_owned(),
        // summary_embedding: vec![0.1, 2.2, 3.4, 9.1, 0.3],
        burns: HashMap::new(),
    };

    let chunks = vec![
        DBDocumentChunk {
            parent_url: url.clone(),
            content: "This is chunk 1 content".to_owned(),
            content_embedding: vec![1.1, 2.3, 92.0, 3.4, 3.3],
            range: (0, 1),
        },
        DBDocumentChunk {
            parent_url: url.clone(),
            content: "This is chunk 2 content".to_owned(),
            content_embedding: vec![1.1, 2.3, 92.0, 3.4, 3.3],
            range: (1, 2),
        },
        DBDocumentChunk {
            parent_url: url.clone(),
            content: "This is chunk 3 content".to_owned(),
            content_embedding: vec![1.1, 2.3, 92.0, 3.4, 3.3],
            range: (2, 3),
        },
        DBDocumentChunk {
            parent_url: url.clone(),
            content: "This is chunk 4 content".to_owned(),
            content_embedding: vec![1.1, 2.3, 92.0, 3.4, 3.3],
            range: (3, 4),
        },
        DBDocumentChunk {
            parent_url: url.clone(),
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
    // assert_eq!(doc.summary, got_doc.unwrap().summary);

    let _ = db.remove_doc_by_url(&doc.url).await;
    let _ = db.remove_chunks_by_url(&doc.url).await;

    let got_chunks = db.get_chunks_by_url(&doc.url).await.unwrap();
    assert_eq!(0, got_chunks.len());
    let got_doc = db.get_doc_by_url(&doc.url).await.unwrap();
    assert!(got_doc.is_none());
    assert!(db.kill_handle().await.is_ok());
}
