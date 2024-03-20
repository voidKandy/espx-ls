use std::collections::HashMap;

use crate::espx_env::agents::{get_indy_agent, independent::IndyAgent};

use super::super::{chunks::DocumentChunk, Document};
use lsp_types::Url;
use serde::{Deserialize, Serialize};

// pub type DBDocumentTuple = (DBDocument, Vec<DBDocumentChunk>);
//
// impl From<DBDocumentTuple> for Document {
//     fn from((dbdoc, dbdoc_chunks): DBDocumentTuple) -> Self {
//         let chunks = dbdoc_chunks
//             .into_iter()
//             .map(|ch| DocumentChunk {
//                 range: ch.range,
//                 content: ch.content.to_owned(),
//                 summary: Some(ch.summary.to_owned()),
//                 changes: HashMap::new(),
//             })
//             .collect();
//         Document {
//             url: dbdoc.url,
//             chunks,
//             summary: Some(dbdoc.summary.to_owned()),
//         }
//     }
// }
//
// pub(super) async fn doc_as_db_tuple(doc: &mut Document) -> Result<DBDocumentTuple, anyhow::Error> {
//     if doc.summary.is_none() {
//         doc.get_summary().await?;
//     }
//     let embedder = get_indy_agent(IndyAgent::Summarizer).unwrap();
//     let summary = doc.summary.as_ref().unwrap().to_owned();
//     let summary_embedding = embedder.get_embedding(&summary).await.unwrap();
//     let dbdoc = DBDocument {
//         url: doc.url.clone(),
//         summary,
//         summary_embedding,
//     };
//
//     let mut dbdoc_chunks = vec![];
//     for ch in doc.chunks.iter_mut() {
//         if ch.summary.is_none() {
//             ch.get_summary().await?;
//         }
//         let summary = ch.summary.as_ref().unwrap().to_owned();
//         let summary_embedding = embedder.get_embedding(&summary).await.unwrap();
//
//         let content_embedding = embedder.get_embedding(&ch.content).await.unwrap();
//
//         let dbchunk = DBDocumentChunk {
//             parent_url: doc.url.clone(),
//             content: ch.content.to_owned(),
//             content_embedding,
//             summary,
//             summary_embedding,
//             range: ch.range,
//         };
//         dbdoc_chunks.push(dbchunk);
//     }
//     Ok((dbdoc, dbdoc_chunks))
// }
