pub mod lru;
use anyhow::anyhow;
use lsp_types::{TextDocumentContentChangeEvent, Url};
use once_cell::sync::Lazy;
use std::{
    collections::HashMap,
    sync::{Arc, RwLock},
};

use self::lru::LRUCache;

pub static GLOBAL_CACHE: Lazy<Arc<RwLock<GlobalCache>>> = Lazy::new(GlobalCache::init_ref_counted);

#[derive(Debug, PartialEq, Eq, Hash, Clone)]
struct ChangesLookup {
    line: usize,
    char_idx: usize,
    char: char,
}

pub struct GlobalCache {
    /// This should be changed to an LRU
    changes_lru: LRUCache<Url, Vec<ChangesLookup>>,
    docs_lru: LRUCache<Url, String>,
}

impl GlobalCache {
    pub fn init_ref_counted() -> Arc<RwLock<Self>> {
        Arc::new(RwLock::new(Self {
            changes_lru: LRUCache::new(5),
            docs_lru: LRUCache::new(5),
        }))
    }

    pub fn changes_at_capacity(&self) -> bool {
        self.changes_lru.at_capacity()
    }

    pub fn docs_at_capacity(&self) -> bool {
        self.docs_lru.at_capacity()
    }

    pub fn get_doc(&mut self, url: &Url) -> Option<String> {
        self.docs_lru.get(url)
    }

    pub fn update_changes(
        &mut self,
        event: &TextDocumentContentChangeEvent,
        url: Url,
    ) -> Result<(), anyhow::Error> {
        if let Some(range) = event.range {
            let texts: Vec<&str> = event.text.lines().collect();
            let start_char = range.start.character as usize;
            let start_line = range.start.line as usize;

            for (line_number, t) in texts.into_iter().enumerate() {
                let current_line = start_line + line_number;
                let mut current_line_changes_lookup = Vec::new();
                t.char_indices().for_each(|(char_idx, char)| {
                    current_line_changes_lookup.push(ChangesLookup {
                        line: current_line,
                        char_idx: char_idx + start_char,
                        char,
                    })
                });
                match self.changes_lru.get(&url) {
                    Some(mut changes_vec) => {
                        changes_vec.iter_mut().for_each(|change| {
                            if let Some(idx) =
                                current_line_changes_lookup.iter_mut().position(|ch| {
                                    ch.line == change.line && ch.char_idx == change.char_idx
                                })
                            {
                                let overwrite = current_line_changes_lookup.swap_remove(idx);
                                change.char = overwrite.char;
                            }
                        });
                        if !current_line_changes_lookup.is_empty() {
                            changes_vec.append(&mut current_line_changes_lookup);
                        }
                        self.changes_lru.update(url.clone(), changes_vec);
                    }
                    None => {
                        self.changes_lru
                            .update(url.clone(), current_line_changes_lookup);
                    }
                }
            }
            return Ok(());
        }

        Err(anyhow!("No range in change event"))
    }

    pub fn update_doc(&mut self, text: &str, url: Url) {
        self.docs_lru.update(url, text.to_owned());
    }
}
