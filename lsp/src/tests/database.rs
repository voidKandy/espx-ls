// #![allow(unused)]
//
// use crate::{
//     config::DatabaseConfig,
//     state::{
//         burns::{self, Burn, SingleLineActivation},
//         database::{
//             models::{
//                 DBBurn, DBBurnParams, DBChunk, DBChunkParams, DBDocument, DBDocumentParams,
//                 DatabaseStruct,
//             },
//             Database,
//         },
//     },
// };
// use lsp_types::Uri;
// use serde::de::Expected;
// use serde::{Deserialize, Serialize};
// use std::collections::HashMap;
// use std::str::FromStr;
// use std::time::Duration;
// use std::vec;
// use surrealdb::Surreal;
// use surrealdb::{engine::remote::ws::Ws, sql::Thing};
// use tokio::time::sleep;
// use tracing::info;
// use tracing_subscriber::fmt::format::Full;
//
// struct DBDocTestCase {
//     input: (Uri, String),
//     expected: DBDocumentParams,
// }
//
// #[test]
// fn test_db_docs_creation() {
//     for case in setup_doc_tests() {
//         let (uri, text) = case.input;
//         let full = DBDocumentParams::build(&text, uri).unwrap();
//
//         for chunk in full.chunks {
//             assert!(case.expected.chunks.iter().any(|ch| {
//                 // We do not compare content embeddings
//                 ch.uri == chunk.uri && ch.content == chunk.content && ch.range == chunk.range
//             }));
//         }
//         for burn in full.burns {
//             for cb in &case.expected.burns {
//                 println!("burn: {:?}\n expected: {:?}\n", burn, cb);
//                 assert!(cb.burn.activation == burn.burn.activation);
//                 assert!(cb.uri == burn.uri);
//                 assert!(cb.burn.lines() == burn.burn.lines());
//             }
//         }
//
//         assert_eq!(full.uri, case.expected.uri);
//     }
// }
//
// #[tokio::test]
// async fn database_spawn_crud_test() {
//     super::init_test_tracing();
//     let test_conf = DatabaseConfig {
//         port: 8081,
//         namespace: "test".to_owned(),
//         database: "test".to_owned(),
//         host: None,
//         user: None,
//         pass: None,
//     };
//     let mut db = Database::init(&test_conf)
//         .await
//         .expect("Failed to init database");
//     sleep(Duration::from_millis(300)).await;
//
//     DBDocument::take_all(&db).await.unwrap();
//     let mut cases = setup_doc_tests();
//     DBDocument::create_many(&db, cases.iter().map(|c| c.expected.clone()).collect())
//         .await
//         .unwrap();
//     for mut case in cases.iter_mut() {
//         let got_chunks = DBChunk::get_by_field(&db, "uri", &case.input.0)
//             .await
//             .unwrap();
//
//         assert_eq!(got_chunks.len(), case.expected.chunks.len());
//
//         for chunk in got_chunks {
//             assert!(case.expected.chunks.iter().any(|ch| {
//                 ch.content == chunk.content && ch.range == chunk.range && ch.uri == chunk.uri
//             }))
//         }
//
//         let got_burns = DBBurn::get_by_field(&db, "uri", &case.input.0)
//             .await
//             .unwrap();
//
//         assert_eq!(got_burns.len(), case.expected.burns.len());
//         for burn in got_burns {
//             assert!(case.expected.burns.iter().any(|b| {
//                 b.burn == burn.burn && b.uri == burn.uri && b.burn.lines() == burn.burn.lines()
//             }))
//         }
//     }
//
//     let all_docs = DBDocument::get_all(&db).await.unwrap();
//     assert_eq!(all_docs.len(), setup_doc_tests().len());
//
//     let all_chunks = DBChunk::get_all(&db).await.unwrap();
//     assert_eq!(
//         all_chunks.len(),
//         setup_doc_tests()
//             .iter()
//             .fold(0, |acc, case| { acc + case.expected.chunks.len() })
//     );
//
//     let all_burns = DBBurn::get_all(&db).await.unwrap();
//     assert_eq!(
//         all_burns.len(),
//         setup_doc_tests()
//             .iter()
//             .fold(0, |acc, case| { acc + case.expected.burns.len() })
//     );
//
//     DBDocument::take_all(&db).await.unwrap();
//
//     let all_docs = DBDocument::get_all(&db).await.unwrap();
//
//     let all_chunks = DBChunk::get_all(&db).await.unwrap();
//     let all_burns = DBBurn::get_all(&db).await.unwrap();
//     assert_eq!(all_docs.len(), 0);
//     assert_eq!(all_chunks.len(), 0);
//     assert_eq!(all_burns.len(), 0);
//
//     db.kill_handle().await.unwrap();
// }
//
// fn setup_doc_tests() -> Vec<DBDocTestCase> {
//     let mut all = vec![];
//
//     let uri = Uri::from_str("file:///tmp/foo").unwrap();
//     let chunks = vec![
//         r#"
//      This is chunk 1 of foo
//
//
//      #$ There is a burn here
//
//
//
//
//
//
//
//
//
//
//
//
//
//
//
// "#,
//         r#"
//      .............
//      This is chunk 2 of foo
//      ...............
//      "#,
//     ];
//
//     let expected = DBDocumentParams {
//         uri: uri.clone(),
//         chunks: vec![
//             DBChunkParams::new(uri.clone(), 0, 20, chunks[0].to_string()),
//             DBChunkParams::new(uri.clone(), 21, 25, chunks[1].to_string()),
//         ]
//         .into(),
//         burns: vec![DBBurnParams {
//             burn: SingleLineActivation::QuickPrompt(4).into(),
//             uri: uri.clone(),
//         }],
//     };
//     all.push(DBDocTestCase {
//         input: (uri, chunks.join("\n").to_string()),
//         expected,
//     });
//
//     let uri = Uri::from_str("file:///tmp/bar").unwrap();
//     let chunks = vec![
//         r#"
//      This is chunk 1 of bar
//
//
//      @@
//
//
//
//
//
//
//
//
//
//
//
//
//
//
//
// "#,
//         r#"
//      .............
//      This is chunk 2 of bar
//      ...............
//      "#,
//     ];
//
//     let expected = DBDocumentParams {
//         uri: uri.clone(),
//         chunks: vec![
//             DBChunkParams::new(uri.clone(), 0, 20, chunks[0].to_string()),
//             DBChunkParams::new(uri.clone(), 21, 25, chunks[1].to_string()),
//         ]
//         .into(),
//
//         burns: vec![DBBurnParams {
//             burn: SingleLineActivation::WalkProject(4).into(),
//             uri: uri.clone(),
//         }],
//     };
//     all.push(DBDocTestCase {
//         input: (uri, chunks.join("\n").to_string()),
//         expected,
//     });
//
//     let uri = Uri::from_str("file:///tmp/baz").unwrap();
//     let chunks = vec!["baz is very small"];
//
//     let expected = DBDocumentParams {
//         uri: uri.clone(),
//         chunks: vec![DBChunkParams::new(uri.clone(), 0, 0, chunks[0].to_string())].into(),
//         burns: vec![],
//     };
//     all.push(DBDocTestCase {
//         input: (uri, chunks.join("\n").to_string()),
//         expected,
//     });
//
//     all
// }
