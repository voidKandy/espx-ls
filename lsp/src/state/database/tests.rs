#![allow(unused)]

use super::models::burns::DBDocumentBurn;
use super::models::full::FullDBDocument;
use super::{
    models::{chunks::ChunkVector, DatabaseStruct},
    DBDocumentChunk, Database,
};
use crate::config::DatabaseConfig;
use crate::error::init_test_tracing;
use crate::state::burns;
use crate::state::database::models::chunks::chunk_vec_from_text;
use lsp_types::Uri;
use serde::de::Expected;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::str::FromStr;
use std::time::Duration;
use std::vec;
use surrealdb::engine::remote::ws::Ws;
use surrealdb::Surreal;
use tokio::time::sleep;
use tracing::info;

struct DBTestCase {
    input: (Uri, String),
    expected: FullDBDocument,
}

#[test]
fn test_db_docs_creation() {
    for mut case in setup_tests() {
        let (uri, text) = case.input;
        let burns: Vec<DBDocumentBurn> = burns::all_activations_in_text(&text).into_iter().fold(
            vec![],
            |mut acc, (lines, b)| {
                acc.push(DBDocumentBurn::new(uri.clone(), lines, b));
                acc
            },
        );

        let chunks = chunk_vec_from_text(uri.clone(), &text).unwrap();
        let full = FullDBDocument {
            id: uri,
            burns,
            chunks,
        };

        for chunk in full.chunks {
            assert!(case.expected.chunks.iter().any(|ch| {
                // We do not compare content embeddings
                ch.parent_uri == chunk.parent_uri
                    && ch.content == chunk.content
                    && ch.range == chunk.range
            }));
        }
        for burn in full.burns {
            for cb in &case.expected.burns {
                assert!(cb.activation == burn.activation);
                assert!(cb.uri == burn.uri);
                assert!(cb.lines == burn.lines);
            }
        }

        assert_eq!(full.id, case.expected.id);
    }
}

#[tokio::test]
async fn database_spawn_crud_test() {
    init_test_tracing();
    let test_conf = DatabaseConfig {
        port: 8081,
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

    FullDBDocument::take_all(&db).await.unwrap();
    for mut case in setup_tests().iter_mut() {
        case.expected.insert(&db).await.unwrap();

        let got_chunks = DBDocumentChunk::get_by_field(&db, "parent_uri", &case.input.0)
            .await
            .unwrap();

        assert_eq!(got_chunks.len(), case.expected.chunks.len());

        for chunk in got_chunks {
            assert!(case.expected.chunks.iter().any(|ch| {
                ch.content == chunk.content
                    && ch.range == chunk.range
                    && ch.parent_uri == chunk.parent_uri
            }))
        }

        let got_burns = DBDocumentBurn::get_by_field(&db, "uri", &case.input.0)
            .await
            .unwrap();

        assert_eq!(got_burns.len(), case.expected.burns.len());
        for burn in got_burns {
            assert!(case.expected.burns.iter().any(|b| {
                b.activation == burn.activation && b.uri == burn.uri && b.lines == burn.lines
            }))
        }
    }

    let all_docs = FullDBDocument::get_all(&db).await.unwrap();
    assert_eq!(all_docs.len(), setup_tests().len());

    let all_chunks = DBDocumentChunk::get_all(&db).await.unwrap();
    assert_eq!(
        all_chunks.len(),
        setup_tests()
            .iter()
            .fold(0, |acc, case| { acc + case.expected.chunks.len() })
    );

    let all_burns = DBDocumentBurn::get_all(&db).await.unwrap();
    assert_eq!(
        all_burns.len(),
        setup_tests()
            .iter()
            .fold(0, |acc, case| { acc + case.expected.burns.len() })
    );

    FullDBDocument::take_all(&db).await.unwrap();

    let all_docs = FullDBDocument::get_all(&db).await.unwrap();

    let all_chunks = DBDocumentChunk::get_all(&db).await.unwrap();
    let all_burns = DBDocumentBurn::get_all(&db).await.unwrap();
    assert_eq!(all_docs.len(), 0);
    assert_eq!(all_chunks.len(), 0);
    assert_eq!(all_burns.len(), 0);

    db.kill_handle().await.unwrap();
}

fn setup_tests() -> Vec<DBTestCase> {
    let mut all = vec![];

    let uri = Uri::from_str("file:///tmp/foo").unwrap();
    let chunks = vec![
        r#"
     This is chunk 1 of foo


     #$ There is a burn here















"#,
        r#"
     .............
     This is chunk 2 of foo
     ...............
     "#,
    ];

    let expected = FullDBDocument {
        id: uri.clone(),
        chunks: vec![
            DBDocumentChunk::new(uri.clone(), 0, 20, chunks[0].to_string()),
            DBDocumentChunk::new(uri.clone(), 21, 25, chunks[1].to_string()),
        ]
        .into(),
        burns: vec![DBDocumentBurn {
            id: None,
            activation: burns::BurnActivation::Single(burns::SingleLineBurn::QuickPrompt {
                hover_contents: None,
            }),
            uri: uri.clone(),
            lines: vec![4],
        }],
    };
    all.push(DBTestCase {
        input: (uri, chunks.join("\n").to_string()),
        expected,
    });

    let uri = Uri::from_str("file:///tmp/bar").unwrap();
    let chunks = vec![
        r#"
     This is chunk 1 of bar


     @@















"#,
        r#"
     .............
     This is chunk 2 of bar 
     ...............
     "#,
    ];

    let expected = FullDBDocument {
        id: uri.clone(),
        chunks: vec![
            DBDocumentChunk::new(uri.clone(), 0, 20, chunks[0].to_string()),
            DBDocumentChunk::new(uri.clone(), 21, 25, chunks[1].to_string()),
        ]
        .into(),
        burns: vec![DBDocumentBurn {
            id: None,
            activation: burns::BurnActivation::Single(burns::SingleLineBurn::WalkProject {
                hover_contents: None,
            }),
            uri: uri.clone(),
            lines: vec![4],
        }],
    };
    all.push(DBTestCase {
        input: (uri, chunks.join("\n").to_string()),
        expected,
    });

    let uri = Uri::from_str("file:///tmp/baz").unwrap();
    let chunks = vec!["baz is very small"];

    let expected = FullDBDocument {
        id: uri.clone(),
        chunks: vec![DBDocumentChunk::new(
            uri.clone(),
            0,
            0,
            chunks[0].to_string(),
        )]
        .into(),
        burns: vec![],
    };
    all.push(DBTestCase {
        input: (uri, chunks.join("\n").to_string()),
        expected,
    });

    all
}
