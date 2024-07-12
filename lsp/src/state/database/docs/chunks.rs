use super::super::{error::DatabaseError, DatabaseIdentifier};
use crate::{
    embeddings,
    state::database::{error::DatabaseResult, Database, Record},
};
use lsp_types::Uri;
use serde::{Deserialize, Serialize};
use tracing::info;

#[derive(Clone, Serialize, Deserialize, Debug)]
pub struct ChunkVector(Vec<DBDocumentChunk>);

impl From<Vec<DBDocumentChunk>> for ChunkVector {
    fn from(value: Vec<DBDocumentChunk>) -> Self {
        Self(value)
    }
}

impl Into<Vec<DBDocumentChunk>> for ChunkVector {
    fn into(self) -> Vec<DBDocumentChunk> {
        self.0
    }
}

const LINES_IN_CHUNK: u32 = 20;
#[derive(Clone, Serialize, Deserialize, Debug)]
pub struct DBDocumentChunk {
    pub parent_uri: Uri,
    pub content: String,
    pub content_embedding: Vec<f32>,
    pub range: (u32, u32),
}

#[allow(unused)]
fn get_chunk_mut_from_line(vec: &mut ChunkVector, line: u32) -> Option<&mut DBDocumentChunk> {
    vec.0
        .iter_mut()
        .find(|c| c.range.1 == line || c.range.0 == line)
}

#[allow(unused)]
fn get_chunk_ref_from_line(vec: &ChunkVector, line: u32) -> Option<&DBDocumentChunk> {
    vec.0
        .iter()
        .find(|c| c.range.1 == line || c.range.0 == line)
}

/// u32 TUPLE END INDEX _IS_ INCLUSIVE
fn chunk_text(text: &str) -> Vec<((u32, u32), String)> {
    let mut chunks = vec![];
    let mut start = 0;
    let mut end = LINES_IN_CHUNK + start;
    let lines: Vec<&str> = text.lines().collect();
    while let Some(window) = {
        match (lines.get(start as usize), lines.get(end as usize)) {
            (Some(_), Some(_)) => Some(lines[start as usize..=end as usize].to_owned()),
            (Some(_), None) => Some(lines[start as usize..].to_owned()),
            _ => None,
        }
    } {
        chunks.push(((start, start + window.len() as u32 - 1), window.join("\n")));
        start += LINES_IN_CHUNK + chunks.len() as u32;
        end += start;
    }
    chunks
}

impl DatabaseIdentifier for DBDocumentChunk {
    fn db_id() -> &'static str {
        "doc_chunks"
    }
}

impl ToString for DBDocumentChunk {
    fn to_string(&self) -> String {
        format!(
            r#"
        [ START OF CHUNK IN RANGE: {:?} ]

        {}

        [ END OF CHUNK IN RANGE: {:?} ]

        "#,
            self.range, self.content, self.range
        )
    }
}

impl DBDocumentChunk {
    fn new(
        parent_uri: Uri,
        starting_line: u32,
        ending_line: u32,
        content: String,
        content_embedding: Vec<f32>,
    ) -> Result<Self, DatabaseError> {
        Ok(Self {
            parent_uri,
            range: (starting_line, ending_line),
            content_embedding,
            content,
        })
    }
}

impl ChunkVector {
    pub fn as_ref(&self) -> &Vec<DBDocumentChunk> {
        &self.0
    }

    pub fn into_text(&self) -> String {
        self.0
            .iter()
            .map(|ch| ch.content.to_owned())
            .collect::<Vec<String>>()
            .join("\n")
    }

    pub fn chunks_from_text(uri: Uri, text: &str) -> DatabaseResult<Self> {
        let mut chunks = vec![];
        let chunked_text = chunk_text(text);
        let mut embeddings = embeddings::get_passage_embeddings(
            chunked_text.iter().map(|(_, t)| t.as_str()).collect(),
        )?;
        for (range, text) in chunked_text.iter() {
            info!("CHUNKED TEXT");
            let chunk = DBDocumentChunk::new(
                uri.clone(),
                range.0,
                range.1,
                text.to_string(),
                embeddings.remove(0),
            )?;
            chunks.push(chunk);
        }
        Ok(chunks.into())
    }

    pub async fn get_relavent(
        db: &Database,
        embedding: Vec<f32>,
        threshold: f32,
    ) -> DatabaseResult<Self> {
        let query = format!("SELECT * FROM {} WHERE vector::similarity::cosine(content_embedding, $embedding) > {};", DBDocumentChunk::db_id(), threshold );
        let mut response = db
            .client
            .query(query)
            .bind(("embedding", embedding))
            .await?;
        let chunks: Vec<DBDocumentChunk> = response.take(0)?;
        Ok(chunks.into())
    }

    pub fn from_text(uri: Uri, text: &str) -> DatabaseResult<Self> {
        let mut chunks = vec![];
        let chunked_text = chunk_text(text);
        let mut embeddings = embeddings::get_passage_embeddings(
            chunked_text.iter().map(|(_, t)| t.as_str()).collect(),
        )?;
        for (range, text) in chunked_text.iter() {
            info!("CHUNKED TEXT");
            let chunk = DBDocumentChunk::new(
                uri.clone(),
                range.0,
                range.1,
                text.to_string(),
                embeddings.remove(0),
            )?;
            chunks.push(chunk);
        }
        Ok(chunks.into())
    }

    pub async fn insert(&self, db: &Database) -> DatabaseResult<Vec<Record>> {
        let mut records = vec![];
        for chunk in self.0.iter() {
            records.append(
                &mut db
                    .client
                    .create(DBDocumentChunk::db_id())
                    .content(chunk)
                    .await?,
            )
        }
        Ok(records)
    }

    pub async fn remove_multiple_by_uri(db: &Database, uri: &Uri) -> DatabaseResult<()> {
        let query = format!(
            "DELETE {} WHERE parent_uri = $uri",
            DBDocumentChunk::db_id()
        );

        db.client.query(query).bind(("uri", uri)).await?;
        Ok(())
    }

    pub async fn get_by_uri(db: &Database, uri: &Uri) -> DatabaseResult<Self> {
        let query = format!(
            "SELECT * FROM {} WHERE parent_uri == $uri",
            DBDocumentChunk::db_id()
        );
        let mut response = db.client.query(query).bind(("uri", uri)).await?;
        let chunks: Vec<DBDocumentChunk> = response.take(0)?;
        Ok(chunks.into())
    }
}
