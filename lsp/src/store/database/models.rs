use std::collections::HashMap;

use anyhow::anyhow;
use lsp_types::{TextDocumentContentChangeEvent, Url};
use serde::{Deserialize, Serialize};

#[derive(Clone, Serialize, Deserialize, Debug)]
pub struct DBDocument {
    pub(super) url: Url,
    pub(super) summary: String,
    pub(super) summary_embedding: Vec<f32>,
}

#[derive(Clone, Serialize, Deserialize, Debug)]
pub struct DBDocumentChunk {
    pub(super) parent_url: Url,
    pub(super) content: String,
    pub(super) content_embedding: Vec<f32>,
    pub(super) summary: String,
    pub(super) summary_embedding: Vec<f32>,
    pub(super) range: (usize, usize),
}

pub type ChunkVector = Vec<DBDocumentChunk>;

fn get_chunk_mut_from_line(vec: &mut ChunkVector, line: usize) -> Option<&mut DBDocumentChunk> {
    vec.iter_mut()
        .find(|c| c.range.1 == line || c.range.0 == line)
}

fn get_chunk_ref_from_line(vec: &ChunkVector, line: usize) -> Option<&DBDocumentChunk> {
    vec.iter().find(|c| c.range.1 == line || c.range.0 == line)
}

impl DBDocumentChunk {
    pub fn db_id() -> &'static str {
        "doc_chunks"
    }
}

impl DBDocument {
    pub fn db_id() -> &'static str {
        "documents"
    }

    // pub fn get_my_chunks(&self) -> ChunkVector {}
    pub fn update(&mut self, event: &TextDocumentContentChangeEvent) -> Result<(), anyhow::Error> {
        Ok(())
        // if let Some(range) = event.range {
        //     let texts: Vec<&str> = event.text.lines().collect();
        //     let start_char = range.start.character as usize;
        //     let start_line = range.start.line as usize;
        //     for (line_number, t) in texts.into_iter().enumerate() {
        //         let current_line = start_line + line_number;
        //         if let Some(chunk) = self.get_chunk_mut_from_line(current_line) {
        //             t.char_indices().for_each(|(char_idx, char)| {
        //                 match chunk.changes.get_mut(&current_line) {
        //                     Some(line_changes_map) => {
        //                         line_changes_map.insert(start_char + char_idx, char);
        //                     }
        //                     None => {
        //                         let mut map = HashMap::new();
        //                         map.insert(start_char + char_idx, char);
        //                         chunk.changes.insert(current_line, map);
        //                     }
        //                 }
        //             })
        //         } else {
        //             return Err(anyhow!("Line: {} had no associated chunk", current_line));
        //         }
        //     }
        //     return Ok(());
        // }
        // Err(anyhow!("No range in change event"))
    }
}
