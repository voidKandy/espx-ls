use std::{
    collections::HashMap,
    sync::{Arc, Mutex},
};

use anyhow::anyhow;
use espionox::environment::agent::{
    language_models::embed,
    memory::{embeddings::EmbeddingVector, MessageRole, ToMessage},
};
use lsp_types::{TextDocumentContentChangeEvent, Url};

#[derive(Debug, Clone)]
pub struct DocumentStore(pub(super) HashMap<Url, (EmbeddingVector, Document)>);

#[derive(Debug, Clone)]
pub struct Document {
    pub url: String,
    pub summary: String,
    // pub chunks: DocumentChunk,
    pub current_text: String,
    pub changes: HashMap<u32, Vec<DocumentChange>>,
}

// #[derive(Debug, Clone)]
// pub struct DocumentChunk {}

#[derive(Debug, Clone)]
pub struct DocumentChange(u32, char);

impl Default for DocumentStore {
    fn default() -> Self {
        Self(HashMap::new())
    }
}

impl DocumentStore {
    /// Takes input vector and proximity value, returns hashmap of urls & docs
    pub fn get_by_proximity(
        &self,
        input_vector: EmbeddingVector,
        proximity: f32,
    ) -> HashMap<&Url, &Document> {
        let mut map = HashMap::new();
        self.0.iter().for_each(|(url, (e, doc))| {
            if input_vector.score_l2(e) <= proximity {
                map.insert(url, doc);
            }
        });
        map
    }

    pub fn update_doc_current_text(
        &mut self,
        uri: &Url,
        current: &str,
    ) -> Result<(), anyhow::Error> {
        self.0
            .get_mut(uri)
            .ok_or(anyhow!("No document with that url"))?
            .1
            .current_text = current.to_string();
        Ok(())
    }

    pub fn insert_or_update(&mut self, doc: Document, url: Url) -> Result<(), anyhow::Error> {
        // Summaries need to be handled but im not sure where
        let embedding = EmbeddingVector::from(embed(&doc.current_text)?);
        match self.0.get_mut(&url) {
            Some((e, d)) => {
                *e = embedding;
                *d = doc;
            }
            None => {
                self.0.insert(url, (embedding, doc));
            }
        }
        Ok(())
    }
}

impl From<(&Url, String)> for Document {
    fn from((url, current_text): (&Url, String)) -> Self {
        Self {
            url: url.to_string(),
            // THIS NEEDS TO BE HANDLED
            summary: "".to_string(),
            current_text,
            changes: HashMap::new(),
        }
    }
}

impl Document {
    pub fn update(&mut self, event: &TextDocumentContentChangeEvent) {
        if let Some(range) = event.range {
            let texts: Vec<&str> = event.text.split('\n').collect();
            let start_char = range.start.character;
            let start_line = range.start.line;
            texts.into_iter().enumerate().for_each(|(line_number, t)| {
                let key = start_line + line_number as u32;
                match self.changes.get_mut(&key) {
                    Some(line_changes) => {
                        t.chars().enumerate().into_iter().for_each(|(char_idx, c)| {
                            line_changes.push(DocumentChange(start_char + char_idx as u32, c))
                        })
                    }

                    None => {
                        let changes = t
                            .chars()
                            .enumerate()
                            .into_iter()
                            .map(|(char_idx, c)| DocumentChange(start_char + char_idx as u32, c))
                            .collect();
                        self.changes.insert(key, changes);
                    }
                }
            })
        }
    }
}

impl ToMessage for Document {
    fn to_message(&self, role: MessageRole) -> espionox::environment::agent::memory::Message {
        let mut lines_changed: Vec<&u32> = self.changes.keys().collect();
        let line_changes_strings: Vec<String> = lines_changed
            .iter_mut()
            .map(|line_no| {
                let mut changes_on_line = self.changes.get(line_no).unwrap().clone();
                changes_on_line.sort_by(|a, b| a.0.cmp(&b.0));
                format!(
                    "[BEGINNING OF CHANGES ON LINE {}]Change starts at character index {} and ends at character index {}[BEGINNING OF CHANGES TEXT]{}[END OF CHANGES TEXT][END OF CHANGES ON LINE {}]",
                    line_no,
                    changes_on_line.first().unwrap().0,
                    changes_on_line.last().unwrap().0,
                    changes_on_line
                        .iter()
                        .fold(String::new(), |mut acc, doc_change| {
                            acc.push(doc_change.1);
                            acc
                        }),
                    line_no
                )
            })
            .collect();
        let content = match line_changes_strings.is_empty() {
            true => {
                format!(
                "This is the current state of the document: {} [BEGINNING OF DOCUMENT]{}[END OF DOCUMENT]",
                self.url, self.current_text
                )
            }
            false => {
                format!(
                "This is the current state of the document: {} [BEGINNING OF DOCUMENT]{}[END OF DOCUMENT]These are the changes that have been made: [BEGINNING OF CHANGES]{:?}[END OF CHANGES]",
                self.url, self.current_text, line_changes_strings
                )
            }
        };
        espionox::environment::agent::memory::Message { role, content }
    }
}

#[cfg(test)]
mod tests {
    use std::collections::HashMap;

    use lsp_types::{Position, TextDocumentContentChangeEvent, Url};

    use crate::store::{
        get_text_document, init_store, update_document_store_from_change_event, GLOBAL_STORE,
    };

    use super::Document;

    #[test]
    fn render_changed_doc_works() {
        //         init_store();
        //         let og = r#"
        // This is the original document
        //
        // there is text here
        //
        //
        // And text here
        //
        // Text here as well
        //             "#;
        //
        //         let uri = Url::parse("file:///tmp/foo").unwrap();
        //         GLOBAL_STORE
        //             .get()
        //             .unwrap()
        //             .lock()
        //             .unwrap()
        //             .insert(uri.clone(), Document::from((&uri, og.to_owned())));
        //
        //         let changes = vec![TextDocumentContentChangeEvent {
        //             range: None,
        //             range_length: None,
        //             text: "a".to_string(),
        //         }];
        //         changes
        //             .into_iter()
        //             .for_each(|c| update_document_store_from_change_event(&uri, &c).unwrap());
        //
        //         // println!("CHANGES {}", changes);
        //         assert!(false);
    }
}
