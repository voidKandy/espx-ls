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
    current_text: String,
    change_counter: u32,
    changes: VecDeque<(SystemTime, String)>,
}

pub static DOCUMENT_STORE: OnceLock<Arc<Mutex<DocumentStore>>> = OnceLock::new();
// Since the LSP receives updates every character, updates need to be pushed every *n* characters
static CHANGE_TRIGGER: u32 = 10;

impl From<String> for Document {
    fn from(current_text: String) -> Self {
        // We dont want a massive vec of changes, so we'll use a ring buffer
        let changes = VecDeque::with_capacity(5);
        Self {
            current_text,
            change_counter: 0,
            changes,
        }
    }
}

impl Document {
    fn push_change(&mut self, change: String) {
        let now = SystemTime::now();
        self.changes.push_front((now, change));
    }
}

impl ToMessage for Document {
    fn to_message(&self, role: MessageRole) -> espionox::environment::agent::memory::Message {
        let content = format!(
            " This is the current state of the document: [BEGINNING OF DOCUMENT]{}[END OF DOCUMENT]These are the last 5 changes made: [BEGINNING OF CHANGES]{:?}[END OF CHANGES]",
            self.current_text, self.changes
        );
        espionox::environment::agent::memory::Message { role, content }
    }
}

pub fn init_text_store() {
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
        // doc.update(change)?;
        doc.change_counter += 1;
        if doc.change_counter == CHANGE_TRIGGER {
            doc.push_change(change.text.to_owned());
            doc.change_counter = 0;
        }
        return Ok(());
    }
    return Err(anyhow!("No text document at URL: {:?}", uri));
}

#[cfg(test)]
mod tests {
    use std::collections::HashMap;

    use lsp_types::{Position, TextDocumentContentChangeEvent, Url};

    use crate::text_store::{get_text_document, init_text_store, update_doc_store, DOCUMENT_STORE};

    use super::Document;

    #[test]
    fn render_changed_doc_works() {
        init_text_store();
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
            .insert(uri.clone(), Document::from(og.to_owned()));

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
