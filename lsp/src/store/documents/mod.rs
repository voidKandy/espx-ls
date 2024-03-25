pub mod chunks;
pub mod documents;
use anyhow::anyhow;
pub use documents::Document;
use lsp_types::Url;
use std::collections::HashMap;

use espionox::agents::memory::{Message, MessageRole, ToMessage};

use self::chunks::DocumentChunk;

#[derive(Debug, Clone)]
pub struct DocumentStore(pub(super) HashMap<Url, Document>);

#[derive(Debug, Clone)]
pub struct DocUrlTup(pub Url, pub Document);

impl DocUrlTup {
    pub fn new(url: Url, doc: Document) -> Self {
        Self(url, doc)
    }
}

impl Default for DocumentStore {
    fn default() -> Self {
        Self(HashMap::new())
    }
}

impl DocumentStore {
    // This is very expensive to be running on every save
    pub async fn update_doc_current_text(
        &mut self,
        uri: &Url,
        current: &str,
    ) -> Result<(), anyhow::Error> {
        let chunks = DocumentChunk::chunks_from_text(current);
        self.0
            .get_mut(uri)
            .ok_or(anyhow!("No doc with that url"))?
            .chunks = chunks;
        Ok(())
    }

    pub fn insert_or_update(&mut self, doc_url_tup: DocUrlTup) -> Result<(), anyhow::Error> {
        match self.0.get_mut(&doc_url_tup.0) {
            Some(d) => {
                *d = doc_url_tup.1;
            }
            None => {
                self.0.insert(doc_url_tup.0, doc_url_tup.1);
            }
        }
        Ok(())
    }
}



impl ToMessage for DocUrlTup {
    fn to_message(&self, role: MessageRole) -> Message {
        let mut lines_changed = vec![];
        self.1.chunks.iter().for_each(|ch| {
            ch.changes.keys().for_each(|line_no| {
                lines_changed.push(line_no);
            })
        });
        let line_changes_strings: Vec<String> = lines_changed
            .iter_mut()
            .filter_map(|line_no| {
                if let Some(changes_on_line) = self.1.get_chunk_ref_from_line(**line_no).unwrap().changes.get(*line_no) {
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
                self.0, self.1.content()
                )
            }
            false => {
                format!(
                "This is the current state of the document: {} [BEGINNING OF DOCUMENT]{}[END OF DOCUMENT]These are the changes that have been made: [BEGINNING OF CHANGES]{:?}[END OF CHANGES]",
                self.0, self.1.content(), line_changes_strings
                )
            }
        };
        Message { role, content }
    }
}
