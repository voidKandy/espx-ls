use std::collections::HashMap;

use super::{chunks::*, DocUrlTup};

use anyhow::anyhow;
use lsp_types::{TextDocumentContentChangeEvent, Url};

#[derive(Debug, Clone)]
pub struct Document {
    // pub url: Url,
    pub chunks: Vec<DocumentChunk>,
    // Summary is only generated when stored in LTM
    pub summary: Option<String>,
}

impl Document {
    pub fn get_chunk_ref_from_line(&self, line: usize) -> Option<&DocumentChunk> {
        self.chunks
            .iter()
            .find(|c| c.range.1 == line || c.range.0 == line)
    }

    pub fn update(&mut self, event: &TextDocumentContentChangeEvent) -> Result<(), anyhow::Error> {
        if let Some(range) = event.range {
            let texts: Vec<&str> = event.text.lines().collect();
            let start_char = range.start.character as usize;
            let start_line = range.start.line as usize;
            for (line_number, t) in texts.into_iter().enumerate() {
                let current_line = start_line + line_number;
                if let Some(chunk) = self.get_chunk_mut_from_line(current_line) {
                    t.char_indices().for_each(|(char_idx, char)| {
                        match chunk.changes.get_mut(&current_line) {
                            Some(line_changes_map) => {
                                line_changes_map.insert(start_char + char_idx, char);
                            }
                            None => {
                                let mut map = HashMap::new();
                                map.insert(start_char + char_idx, char);
                                chunk.changes.insert(current_line, map);
                            }
                        }
                    })
                } else {
                    return Err(anyhow!("Line: {} had no associated chunk", current_line));
                }
            }
            return Ok(());
        }
        Err(anyhow!("No range in change event"))
    }

    pub fn new(url: Url, text: &str) -> DocUrlTup {
        let chunks = DocumentChunk::chunks_from_text(text);
        DocUrlTup(
            url,
            Self {
                chunks,
                summary: None,
            },
        )
    }

    pub fn content(&self) -> String {
        self.chunks
            .iter()
            .fold(String::new(), |mut content, chunk| {
                content = format!("{}\n{}", content, chunk.content);
                content
            })
    }

    fn get_chunk_mut_from_line(&mut self, line: usize) -> Option<&mut DocumentChunk> {
        self.chunks
            .iter_mut()
            .find(|c| c.range.1 == line || c.range.0 == line)
    }
}
