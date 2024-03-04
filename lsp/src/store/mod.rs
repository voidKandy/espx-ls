mod actions;
mod documents;
use std::{
    collections::HashMap,
    sync::{Arc, Mutex, OnceLock},
};

pub use actions::*;
use anyhow::anyhow;
pub use documents::*;
use lsp_types::{TextDocumentContentChangeEvent, Url};

#[derive(Debug, Clone, Default)]
pub struct GlobalStore {
    pub documents: DocumentStore,
    // TODO! DOCUMENTATION
    pub actions: ActionStore,
}

pub static GLOBAL_STORE: OnceLock<Arc<Mutex<GlobalStore>>> = OnceLock::new();

pub fn init_store() {
    _ = GLOBAL_STORE.set(Arc::new(Mutex::new(GlobalStore::default())));
}

pub fn get_text_document_current(uri: &Url) -> Option<String> {
    todo!();
    // Some(
    //     GLOBAL_STORE
    //         .get()
    //         .expect("global store not initialized")
    //         .lock()
    //         .expect("global store mutex poisoned")
    //         .documents
    //         .0
    //         .get(uri)?
    //         .1
    //         .current_text
    //         .clone(),
    // )
}

pub fn get_text_document(uri: &Url) -> Option<Document> {
    todo!();
    // Some(
    //     GLOBAL_STORE
    //         .get()
    //         .expect("global store not initialized")
    //         .lock()
    //         .expect("global store mutex poisoned")
    //         .documents
    //         .0
    //         .get(uri)?
    //         .1
    //         .clone(),
    // )
}

pub fn set_doc_current(uri: &Url, current: &str) -> Result<(), anyhow::Error> {
    GLOBAL_STORE
        .get()
        .unwrap()
        .lock()
        .unwrap()
        .documents
        .update_doc_current_text(uri, current)
}

pub fn update_document_store_from_change_event(
    uri: &Url,
    change: &TextDocumentContentChangeEvent,
) -> Result<(), anyhow::Error> {
    let mut store = GLOBAL_STORE.get().unwrap().lock().unwrap();
    if let Some((_, doc)) = store.documents.0.get_mut(&uri) {
        doc.update(change);
        return Ok(());
    }
    return Err(anyhow::anyhow!("No text document at URL: {:?}", uri));
}
