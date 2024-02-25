use std::{
    collections::{HashMap, VecDeque},
    sync::{Arc, Mutex, OnceLock},
    time::SystemTime,
};

use anyhow::anyhow;
use espionox::environment::agent::memory::{MessageRole, ToMessage};
use lsp_types::{lsif::RangeTag, Position, Range, TextDocumentContentChangeEvent, Url};

/// Hashmap of Uri keys to File content values
type DocumentStore = HashMap<Url, Document>;

#[derive(Debug, Clone)]
pub struct Document {
    url: String,
    current_text: String,
    pub changes: HashMap<u32, Vec<DocumentChange>>,
}

#[derive(Debug, Clone)]
pub struct DocumentChange(u32, char);

pub static DOCUMENT_STORE: OnceLock<Arc<Mutex<DocumentStore>>> = OnceLock::new();

impl From<(&Url, String)> for Document {
    fn from((url, current_text): (&Url, String)) -> Self {
        Self {
            url: url.to_string(),
            current_text,
            changes: HashMap::new(),
        }
    }
}

impl Document {
    fn update(&mut self, event: &TextDocumentContentChangeEvent) {
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

pub fn init_doc_store() {
    _ = DOCUMENT_STORE.set(Arc::new(Mutex::new(HashMap::new())));
}

pub fn get_text_document_current(uri: &Url) -> Option<String> {
    Some(
        DOCUMENT_STORE
            .get()
            .expect("text store not initialized")
            .lock()
            .expect("text store mutex poisoned")
            .get(uri)?
            .current_text
            .clone(),
    )
}

pub fn get_text_document(uri: &Url) -> Option<Document> {
    Some(
        DOCUMENT_STORE
            .get()
            .expect("text store not initialized")
            .lock()
            .expect("text store mutex poisoned")
            .get(uri)?
            .clone(),
    )
}

pub fn set_doc_current(uri: &Url, current: &str) -> Result<(), anyhow::Error> {
    let mut store = DOCUMENT_STORE.get().unwrap().lock().unwrap();
    if let Some(doc) = store.get_mut(&uri) {
        doc.current_text = current.to_owned();
        // doc.changes = HashMap::new();
        return Ok(());
    }
    return Err(anyhow!("No text document at URL: {:?}", uri));
}

pub fn update_doc_store(
    uri: &Url,
    change: &TextDocumentContentChangeEvent,
) -> Result<(), anyhow::Error> {
    let mut store = DOCUMENT_STORE.get().unwrap().lock().unwrap();
    if let Some(doc) = store.get_mut(&uri) {
        doc.update(change);
        return Ok(());
    }
    return Err(anyhow!("No text document at URL: {:?}", uri));
}

#[cfg(test)]
mod tests {
    use std::collections::HashMap;

    use lsp_types::{Position, TextDocumentContentChangeEvent, Url};

    use crate::doc_store::{get_text_document, init_doc_store, update_doc_store, DOCUMENT_STORE};

    use super::Document;

    #[test]
    fn render_changed_doc_works() {
        init_doc_store();
        let og = r#"
This is the original document

there is text here 


And text here

Text here as well
            "#;

        let uri = Url::parse("file:///tmp/foo").unwrap();
        DOCUMENT_STORE
            .get()
            .unwrap()
            .lock()
            .unwrap()
            .insert(uri.clone(), Document::from((&uri, og.to_owned())));

        let changes = vec![TextDocumentContentChangeEvent {
            range: None,
            range_length: None,
            text: "a".to_string(),
        }];
        changes
            .into_iter()
            .for_each(|c| update_doc_store(&uri, &c).unwrap());

        // println!("CHANGES {}", changes);
        assert!(false);
    }
}
