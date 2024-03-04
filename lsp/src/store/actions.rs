use std::{
    collections::{HashMap, VecDeque},
    sync::{Arc, Mutex, OnceLock},
    time::SystemTime,
};

use anyhow::anyhow;
use espionox::agents::{
    language_models::embed,
    memory::{embeddings::EmbeddingVector, MessageRole, ToMessage},
};
use lsp_types::{lsif::RangeTag, Position, Range, TextDocumentContentChangeEvent, Url};

#[derive(Debug, Clone)]
pub struct ActionStore(Vec<(EmbeddingVector, Url, Action)>);

#[derive(Debug, Clone)]
pub struct Action {
    summary: String,
    timestamp: SystemTime,
}

impl Default for ActionStore {
    fn default() -> Self {
        Self(vec![])
    }
}

impl ActionStore {
    pub fn get_by_proximity(
        &self,
        input_vector: EmbeddingVector,
        proximity: f32,
    ) -> HashMap<&Url, &Action> {
        let mut map = HashMap::new();
        self.0.iter().for_each(|(e, url, action)| {
            if input_vector.score_l2(e) <= proximity {
                map.insert(url, action);
            }
        });
        map
    }

    pub fn insert_action(&mut self, action: Action, url: Url) -> Result<(), anyhow::Error> {
        let embedding = EmbeddingVector::from(embed(&action.summary)?);
        self.0.push((embedding, url, action));
        Ok(())
    }
}
