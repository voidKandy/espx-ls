use super::error::StoreResult;
use anyhow::anyhow;
use espionox::agents::memory::Message;
use lsp_types::Url;
use std::{
    fs,
    path::PathBuf,
    sync::{Arc, RwLock},
};

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

    /// Recursively walks root to update database
    pub(super) fn walk_dir(&self, path: PathBuf) -> StoreResult<Vec<(Url, String)>> {
        let mut return_vec = vec![];
        if let Ok(read_dir) = fs::read_dir(path.clone()) {
            for entry in read_dir
                .map(|res| res.map(|e| e.path()))
                .flatten()
                .collect::<Vec<PathBuf>>()
                .into_iter()
            {
                match entry.is_dir() {
                    true => {
                        if let Some(mut vec) = self.walk_dir(entry).ok() {
                            if !vec.is_empty() {
                                return_vec.append(&mut vec);
                            }
                        }
                    }
                    false => {
                        let url =
                            Url::parse(entry.to_str().expect("Why couldn't It get str from path?"))
                                .map_err(|err| {
                                    anyhow!("Could not parse URL from entry: {:?}", err)
                                })?;
                        let text = fs::read_to_string(entry)?;
                        return_vec.push((url, text))
                    }
                }
            }
        } else {
            log::error!("PROBLEM READING DIRECTORY: {:?}", path);
        }
        Ok(return_vec)
    }
}
