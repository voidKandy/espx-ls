use espionox::agents::memory::Message;
use lsp_types::Url;
use std::sync::{Arc, RwLock};

pub(super) const AMT_CHANGES_TO_TRIGGER_UPDATE: usize = 5;
#[derive(Debug)]
pub struct AssistantUpdater {
    pub(super) update_message: Arc<RwLock<Option<Message>>>,
    pub(super) counter: usize,
}

impl Default for AssistantUpdater {
    fn default() -> Self {
        Self {
            update_message: Arc::new(RwLock::new(None)),
            counter: 0,
        }
    }
}

impl AssistantUpdater {
    pub fn clone_message(&self) -> Arc<RwLock<Option<Message>>> {
        Arc::clone(&self.update_message)
    }

    /// Recursively walks root to update model
    fn walk_root() -> Vec<(Url, String)> {
        unimplemented!()
    }
}
