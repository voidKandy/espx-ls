mod burns;
pub mod database;
pub mod error;
mod tests;
use self::{
    database::{docs::FullDBDocument, Database},
    error::StoreResult,
};
use crate::{config::GLOBAL_CONFIG, util::LRUCache};
use burns::BurnCache;
use espionox::agents::memory::{Message, ToMessage};
use log::{debug, error};
use lsp_types::Url;
use std::{fs, path::PathBuf};

#[derive(Debug)]
pub struct GlobalStore {
    docs: DocLRU,
    pub burns: BurnCache,
    pub db: Option<DatabaseStore>,
}

#[derive(Debug)]
pub struct DatabaseStore {
    pub client: Database,
    pub cache: Vec<FullDBDocument>,
}

impl ToMessage for GlobalStore {
    fn to_message(&self, role: espionox::agents::memory::MessageRole) -> Message {
        let mut whole_message = String::from("Here are the most recently accessed documents: ");
        for (url, doc_text) in self.docs.0.into_iter() {
            whole_message.push_str(&format!(
                "[BEGINNNING OF DOCUMENT: {}]\n{}\n[END OF DOCUMENT: {}]\n",
                url.as_str(),
                doc_text,
                url.as_str()
            ));
        }
        debug!("LRU CACHE COERCED TO MESSAGE: {}", whole_message);

        Message {
            role,
            content: whole_message,
        }
    }
}

#[derive(Debug)]
pub struct DocLRU(pub(super) LRUCache<Url, String>);
impl Default for DocLRU {
    fn default() -> Self {
        Self(LRUCache::new(5))
    }
}

impl DatabaseStore {
    pub async fn read_all_docs_to_cache(&mut self) -> anyhow::Result<()> {
        let docs = self.client.get_all_docs().await?;
        self.cache = docs;
        Ok(())
    }
}

impl GlobalStore {
    pub async fn init() -> Self {
        let db = match &GLOBAL_CONFIG.database {
            Some(db_cfg) => match Database::init(db_cfg).await {
                Ok(db) => Some(DatabaseStore {
                    client: db,
                    cache: vec![],
                }),
                Err(err) => {
                    debug!(
                        "PROBLEM INTIALIZING DATABASE IN STATE, RETURNING NONE. ERROR: {:?}",
                        err
                    );
                    None
                }
            },
            None => None,
        };
        Self {
            docs: DocLRU::default(),
            burns: BurnCache::default(),
            db,
        }
    }

    pub fn docs_at_capacity(&self) -> bool {
        self.docs.0.at_capacity()
    }

    pub fn get_doc(&mut self, url: &Url) -> Option<String> {
        self.docs.0.get(url)
    }

    pub fn update_doc(&mut self, text: &str, url: Url) {
        self.docs.0.update(url, text.to_owned());
        // self.increment_quick_agent_updates_counter()
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
