use lsp_types::Url;
use serde::{Deserialize, Serialize};

use crate::espx_env::agents::{get_indy_agent, independent::IndyAgent};

use super::error::DbModelError;

pub type ChunkVector = Vec<DBDocumentChunk>;

const LINES_IN_CHUNK: usize = 20;
#[derive(Clone, Serialize, Deserialize, Debug)]
pub struct DBDocumentChunk {
    pub(super) parent_url: Url,
    pub(super) content: String,
    pub(super) content_embedding: Vec<f32>,
    pub(super) range: (usize, usize),
}

pub fn chunk_vec_content(vec: &ChunkVector) -> String {
    vec.iter()
        .map(|ch| ch.content.to_owned())
        .collect::<Vec<String>>()
        .join("\n")
}

fn get_chunk_mut_from_line(vec: &mut ChunkVector, line: usize) -> Option<&mut DBDocumentChunk> {
    vec.iter_mut()
        .find(|c| c.range.1 == line || c.range.0 == line)
}

fn get_chunk_ref_from_line(vec: &ChunkVector, line: usize) -> Option<&DBDocumentChunk> {
    vec.iter().find(|c| c.range.1 == line || c.range.0 == line)
}

/// USIZE TUPLE END INDEX _IS_ INCLUSIVE
fn chunk_text(text: &str) -> Vec<((usize, usize), String)> {
    let mut chunks = vec![];
    let mut start = 0;
    let mut end = LINES_IN_CHUNK + start;
    let lines: Vec<&str> = text.lines().collect();
    while let Some(window) = {
        match (lines.get(start as usize), lines.get(end)) {
            (Some(_), Some(_)) => Some(lines[start..=end].to_owned()),
            (Some(_), None) => Some(lines[start..].to_owned()),
            _ => None,
        }
    } {
        chunks.push(((start, start + window.len() - 1), window.join("\n")));
        start += LINES_IN_CHUNK + chunks.len();
        end += start;
    }
    chunks
}

impl DBDocumentChunk {
    pub fn db_id() -> &'static str {
        "doc_chunks"
    }

    async fn new(
        parent_url: Url,
        starting_line: usize,
        ending_line: usize,
        content: String,
    ) -> Result<Self, DbModelError> {
        let embedder = get_indy_agent(IndyAgent::Embedder)
            .ok_or(DbModelError::FailedToGetAgent(IndyAgent::Embedder))?;
        let content_embedding = embedder.get_embedding(&content).await?;

        Ok(Self {
            parent_url,
            range: (starting_line, ending_line),
            content_embedding,
            content,
        })
    }
    pub async fn chunks_from_text(url: Url, text: &str) -> Result<ChunkVector, DbModelError> {
        let mut chunks = vec![];
        for (range, text) in chunk_text(text) {
            let chunk = DBDocumentChunk::new(url.clone(), range.0, range.1, text).await?;
            chunks.push(chunk);
        }
        Ok(chunks)
    }
}
