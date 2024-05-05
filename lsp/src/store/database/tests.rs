#![allow(unused)]

use super::{DBDocumentChunk, DBDocumentInfo, Database, FullDBDocument};
use crate::config::DatabaseConfig;
use log::info;
use lsp_types::Url;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::time::Duration;
use structured_logger::json::new_writer;
use structured_logger::Builder;
use surrealdb::engine::remote::ws::Ws;
use surrealdb::Surreal;
use tokio::time::sleep;

fn test_doc_data() -> Vec<FullDBDocument> {
    let mut result = vec![];

    let url = Url::parse("file:///tmp/foo").unwrap();
    let info = DBDocumentInfo {
        url: url.clone(),
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
    result.push(FullDBDocument { info, chunks });

    let url = Url::parse("file:///tmp/foo2").unwrap();
    let info = DBDocumentInfo {
        url: url.clone(),
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
    ];

    result.push(FullDBDocument { info, chunks });
    return result;
}

#[tokio::test]
async fn database_spawn_crud_test() {
    Builder::with_level("debug")
        .with_default_writer(new_writer(std::io::stdout()))
        .init();
    info!("INIT DATABASE TEST");
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
    let test_docs = test_doc_data();

    for test_doc in test_docs.iter() {
        let rec = db.insert_doc_info(&test_doc.info).await;
        info!("DOCUMENT RECORDS: {:?}", rec);

        let rec = db.insert_chunks(&test_doc.chunks).await;
        info!("CHUNKS RECORDS: {:?}", rec);
    }

    let test_doc = &test_docs[0];
    let got_chunks = db.get_chunks_by_url(&test_doc.info.url).await.unwrap();
    assert_eq!(test_doc.chunks.len(), got_chunks.len());

    let got_info = db.get_info_by_url(&test_doc.info.url).await.unwrap();
    assert_eq!(
        test_doc.info.burns.len(),
        got_info.clone().unwrap().burns.len()
    );

    let got_all = db.get_all_docs().await.unwrap();
    assert_eq!(got_info.unwrap().burns.len(), got_all[0].info.burns.len());
    assert_eq!(got_chunks.len(), got_all[0].chunks.len());
    assert_eq!(got_all[1].info.url.as_str(), "file:///tmp/foo2");
    assert_eq!(got_all[1].chunks.len(), 4);

    let _ = db.remove_doc_by_url(&test_doc.info.url).await;
    let _ = db.remove_chunks_by_url(&test_doc.info.url).await;

    let got_chunks = db.get_chunks_by_url(&test_doc.info.url).await.unwrap();
    assert_eq!(0, got_chunks.len());
    let got_doc = db.get_info_by_url(&test_doc.info.url).await.unwrap();
    assert!(db.kill_handle().await.is_ok());
    assert!(got_doc.is_none());
}
