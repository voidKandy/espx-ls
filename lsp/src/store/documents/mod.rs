pub mod chunks;
pub mod documents;
use std::{
    collections::HashMap,
    sync::{Arc, Mutex},
};

use anyhow::anyhow;
use chunks::*;
use documents::*;

pub use documents::Document;
use espionox::agents::{
    language_models::embed,
    memory::{embeddings::EmbeddingVector, MessageRole, ToMessage},
};
use lsp_types::{TextDocumentContentChangeEvent, Url};

#[derive(Debug, Clone)]
pub struct DocumentStore(pub(super) HashMap<Url, (EmbeddingVector, Document)>);

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
        // self.0
        //     .get_mut(uri)
        //     .ok_or(anyhow!("No document with that url"))?
        //     .1
        //     .current_text = current.to_string();
        Ok(())
    }

    pub fn insert_or_update(&mut self, doc: Document, url: Url) -> Result<(), anyhow::Error> {
        // let embedding = EmbeddingVector::from(embed(&doc.current_text)?);
        // match self.0.get_mut(&url) {
        //     Some((e, d)) => {
        //         *e = embedding;
        //         *d = doc;
        //     }
        //     None => {
        //         self.0.insert(url, (embedding, doc));
        //     }
        // }
        Ok(())
    }
}
