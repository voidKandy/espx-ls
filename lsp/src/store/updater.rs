use crate::espx_env::listeners::LRURAG;

use super::{
    error::{StoreError, StoreResult},
    GlobalStore,
};
use espionox::agents::memory::{Message, MessageRole, OtherRoleTo, ToMessage};
use log::{debug, error, info};
use std::{
    fs,
    path::PathBuf,
    sync::{Arc, RwLock},
};

pub(super) const AMT_CHANGES_TO_TRIGGER_UPDATE: usize = 5;
#[derive(Debug, Default)]
pub struct AssistantUpdater {
    pub quick: QuickAssistantUpdater,
    pub db: DBAssistantUpdater,
}

#[derive(Debug, Default)]
pub struct QuickAssistantUpdater {
    pub(super) message: Arc<RwLock<Option<Message>>>,
    pub(super) counter: usize,
}

#[derive(Debug, Default)]
pub struct DBAssistantUpdater {
    pub(super) message: Arc<RwLock<Option<Message>>>,
}

impl DBAssistantUpdater {
    pub fn clone_message(&self) -> Arc<RwLock<Option<Message>>> {
        Arc::clone(&self.message)
    }
}

impl QuickAssistantUpdater {
    pub fn clone_message(&self) -> Arc<RwLock<Option<Message>>> {
        Arc::clone(&self.message)
    }
}

pub fn walk_dir(path: PathBuf) -> StoreResult<Vec<(PathBuf, String)>> {
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
                        match walk_dir(entry) {
                            Ok(mut vec) => {
                                if !vec.is_empty() {
                                    return_vec.append(&mut vec);
                                }
                            }
                            Err(err) => {
                                error!("Encountered error while walking sub-directory: {:?}", err)
                            }
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
