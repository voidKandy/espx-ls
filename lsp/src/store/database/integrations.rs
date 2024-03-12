use std::collections::HashMap;

use super::super::{chunks::DocumentChunk, Document, Summarizable};
use espionox::agents::language_models::embed;
use lsp_types::Url;
use serde::{Deserialize, Serialize};

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

pub(super) async fn doc_as_db_tuple<'db>(
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
