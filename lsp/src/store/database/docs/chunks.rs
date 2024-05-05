use super::super::{error::DBModelError, DatabaseIdentifier};
use crate::embeddings;
use log::info;
use lsp_types::Url;
use serde::{Deserialize, Serialize};

pub type ChunkVector = Vec<DBDocumentChunk>;

const LINES_IN_CHUNK: usize = 20;
#[derive(Clone, Serialize, Deserialize, Debug)]
pub struct DBDocumentChunk {
    pub parent_url: Url,
    pub content: String,
    pub content_embedding: Vec<f32>,
    pub range: (usize, usize),
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

impl DatabaseIdentifier for DBDocumentChunk {
    fn db_id() -> &'static str {
        "doc_chunks"
    }
}

impl DBDocumentChunk {
    fn new(
        parent_url: Url,
        starting_line: usize,
        ending_line: usize,
        content: String,
        content_embedding: Vec<f32>,
    ) -> Result<Self, DBModelError> {
        Ok(Self {
            parent_url,
            range: (starting_line, ending_line),
            content_embedding,
            content,
        })
    }
    pub fn chunks_from_text(url: Url, text: &str) -> Result<ChunkVector, DBModelError> {
        let mut chunks = vec![];
        let chunked_text = chunk_text(text);
        let mut embeddings = embeddings::get_passage_embeddings(
            chunked_text.iter().map(|(_, t)| t.as_str()).collect(),
        )?;
        for (range, text) in chunked_text.iter() {
            info!("CHUNKED TEXT");
            let chunk = DBDocumentChunk::new(
                url.clone(),
                range.0,
                range.1,
                text.to_string(),
                embeddings.remove(0),
            )?;
            chunks.push(chunk);
        }
        Ok(chunks)
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
