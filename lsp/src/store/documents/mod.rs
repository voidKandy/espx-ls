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
pub struct DocumentStore(pub(super) HashMap<Url, (EmbeddingVector, Document)>);

impl Default for DocumentStore {
    fn default() -> Self {
        Self(HashMap::new())
    }
}

pub trait Summarizable {
    async fn get_summary(&mut self) -> Result<(), anyhow::Error>;
}

impl DocumentStore {
    /// Takes input vector and proximity value, returns hashmap of urls & docs
    pub fn get_by_proximity(
        &self,
        input_vector: EmbeddingVector,
        distance: f32,
    ) -> HashMap<&Url, &Document> {
        let (sender, receiver) = channel();

        self.0
            .par_iter()
            .for_each_with(sender, |s, (url, (e, doc))| {
                if input_vector.score_l2(e) <= distance {
                    s.send((url, doc)).expect("Failed to send");
                }
            });

        let mut map = HashMap::new();
        while let Some((url, doc)) = receiver.recv().ok() {
            map.insert(url, doc);
        }

        map
    }

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
            .1
            .chunks = chunks;
        Ok(())
    }

    pub fn insert_or_update(&mut self, doc: Document, url: Url) -> Result<(), anyhow::Error> {
        let embedding = EmbeddingVector::from(embed(&doc.content())?);
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
