use std::collections::HashMap;


use super::chunks::*;

use anyhow::anyhow;
use espionox::agents::memory::{Message, MessageRole, ToMessage};
use lsp_types::{TextDocumentContentChangeEvent, Url};

#[derive(Debug, Clone)]
pub struct Document {
    pub url: Url,
    pub chunks: Vec<DocumentChunk>,
    // Summary is only generated when stored in LTM
    pub summary: Option<String>,
}


impl Document {
    pub  fn new(url: Url, text: &str) -> Self {
        let chunks = DocumentChunk::chunks_from_text(text);
        // let summary = summarize(Some(SUMMARIZE_WHOLE_DOC_PROMPT), text).await?;
        Self {
            url,
            chunks,
            summary: None,
        }
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

    fn get_chunk_ref_from_line(&self, line: usize) -> Option<&DocumentChunk> {
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
}

impl ToMessage for Document {
    fn to_message(&self, role: MessageRole) -> Message {
        let mut lines_changed = vec![];
        self.chunks.iter().for_each(|ch| {
            ch.changes.keys().for_each(|line_no| {
                lines_changed.push(line_no);
            })
        });
        let line_changes_strings: Vec<String> = lines_changed
            .iter_mut()
            .filter_map(|line_no| {
                if let Some(changes_on_line) = self.get_chunk_ref_from_line(**line_no).unwrap().changes.get(*line_no) {
                        let mut changes: Vec<(usize, char)> = changes_on_line.iter().map(|(char_idx, char)| (*char_idx, *char)).collect();
                        changes.sort_by(|a,b| a.0.cmp(&b.0));
                        Some(format!(
                            "[BEGINNING OF CHANGES ON LINE {}]Change starts at character index {} and ends at character index {}[BEGINNING OF CHANGES TEXT]{}[END OF CHANGES TEXT][END OF CHANGES ON LINE {}]",
                            line_no,
                            changes.first().unwrap().0,
                            changes.last().unwrap().0,
                            changes
                            .iter()
                            .fold(String::new(), |mut acc, doc_change| {
                                acc.push(doc_change.1);
                                acc
                            }),
                        line_no
                        ))
                }             
                else {
                    None
                }
            })
            .collect();
        let content = match line_changes_strings.is_empty() {
            true => {
                format!(
                "This is the current state of the document: {} [BEGINNING OF DOCUMENT]{}[END OF DOCUMENT]",
                self.url, self.content()
                )
            }
            false => {
                format!(
                "This is the current state of the document: {} [BEGINNING OF DOCUMENT]{}[END OF DOCUMENT]These are the changes that have been made: [BEGINNING OF CHANGES]{:?}[END OF CHANGES]",
                self.url, self.content(), line_changes_strings
                )
            }
        };
        Message { role, content }
    }
}
