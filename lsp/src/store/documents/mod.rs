pub mod chunks;
pub mod documents;
use anyhow::anyhow;
pub use documents::Document;
use espionox::agents::language_models::embed;
use espionox::agents::memory::embeddings::EmbeddingVector;
use lsp_types::Url;
use std::collections::HashMap;
use std::sync::mpsc::channel;

use rayon::prelude::*;

use self::chunks::DocumentChunk;

#[derive(Debug, Clone)]
pub struct DocumentStore(pub(super) HashMap<Url, Document>);

impl Default for DocumentStore {
    fn default() -> Self {
        Self(HashMap::new())
    }
}

pub trait Summarizable {
    async fn get_summary(&mut self) -> Result<(), anyhow::Error>;
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

    pub fn insert_or_update(&mut self, doc: Document, url: Url) -> Result<(), anyhow::Error> {
        let embedding = EmbeddingVector::from(embed(&doc.content())?);
        match self.0.get_mut(&url) {
            Some(d) => {
                *d = doc;
            }
            None => {
                self.0.insert(url, doc);
            }
        }
        Ok(())
    }
}
