use super::error::StoreResult;
use anyhow::anyhow;
use espionox::agents::memory::Message;
use log::{debug, error};
use lsp_types::Url;
use std::{
    fs,
    path::PathBuf,
    sync::{Arc, RwLock},
};

pub(super) const AMT_CHANGES_TO_TRIGGER_UPDATE: usize = 5;
#[derive(Debug)]
pub struct AssistantUpdater {
    pub(super) update_from_database: bool,
    pub(super) database_update: Arc<RwLock<Option<Message>>>,
    pub(super) in_memory_update: Arc<RwLock<Option<Message>>>,
    pub(super) counter: usize,
}

impl Default for AssistantUpdater {
    fn default() -> Self {
        Self {
            update_from_database: false,
            database_update: Arc::new(RwLock::new(None)),
            in_memory_update: Arc::new(RwLock::new(None)),
            counter: 0,
        }
    }
}

impl AssistantUpdater {
    pub fn clone_message(&self) -> Arc<RwLock<Option<Message>>> {
        Arc::clone(&self.in_memory_update)
    }

    pub fn walk_dir(&self, path: PathBuf) -> StoreResult<Vec<(PathBuf, String)>> {
        debug!("WALKING DIRECTORY: {:?}", path);
        let mut return_vec = vec![];
        match fs::read_dir(path.clone()) {
            Ok(read_dir) => {
                debug!("fs::read_dir returned OK");
                let filtered_entries = read_dir
                    .filter_map(|res| match res {
                        Ok(r) => {
                            if r.path()
                                .as_os_str()
                                .to_string_lossy()
                                .to_string()
                                .split_once('/')
                                .unwrap()
                                .1
                                .chars()
                                .nth(0)
                                != Some('.')
                            {
                                Some(r.path())
                            } else {
                                None
                            }
                        }
                        Err(err) => {
                            error!("PROBLEM WITH READ_DIR RESPONSE: {:?}", err);
                            None
                        }
                    })
                    .collect::<Vec<PathBuf>>();
                debug!("Got {:?} filtered entries", filtered_entries.len());
                for entry in filtered_entries.into_iter() {
                    match entry.is_dir() {
                        true => {
                            debug!("entry is directory");
                            match self.walk_dir(entry) {
                                Ok(mut vec) => {
                                    if !vec.is_empty() {
                                        return_vec.append(&mut vec);
                                    }
                                }
                                Err(err) => error!(
                                    "Encountered error while walking sub-directory: {:?}",
                                    err
                                ),
                            }
                        }
                        false => {
                            debug!("entry is not directory");
                            match fs::read_to_string(entry.clone()) {
                                Ok(text) => return_vec.push((entry, text)),
                                Err(err) => error!(
                                    "Encountered error when reading {:?} to string: {:?}",
                                    entry, err
                                ),
                            }
                        }
                    }
                }
            }
            Err(err) => log::error!(
                "fs::read_dir encountered problem reading directory: {:?}",
                err
            ),
        }
        debug!("Returning vector of urls & texts");
        Ok(return_vec)
    }
}
