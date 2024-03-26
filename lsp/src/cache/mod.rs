pub mod lru;
use anyhow::anyhow;
use espionox::agents::memory::{Message, MessageRole, ToMessage};
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
    pub(super) line: usize,
    pub(super) char_idx: usize,
    pub(super) char: char,
}

#[derive(Debug)]
pub struct GlobalCache {
    changes_lru: LRUCache<Url, Vec<ChangesLookup>>,
    docs_lru: LRUCache<Url, String>,
}

impl ChangesLookup {
    fn to_line_map(mut vec: Vec<ChangesLookup>) -> HashMap<usize, Vec<(usize, char)>> {
        vec.sort_by(|a, b| a.line.cmp(&b.line));
        let mut map = HashMap::<usize, Vec<(usize, char)>>::new();
        while let Some(change) = vec.pop() {
            match map.get_mut(&change.line) {
                Some(vec) => vec.push((change.char_idx, change.char)),
                None => {
                    let _ = map.insert(change.line, vec![(change.char_idx, change.char)]);
                }
            }
        }
        map
    }
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

    pub fn as_message(&self, role: MessageRole) -> Message {
        self.to_message(role)
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

impl ToMessage for GlobalCache {
    fn to_message(&self, role: espionox::agents::memory::MessageRole) -> Message {
        let mut whole_message = String::from("Here are the most recently accessed documents: ");
        for (url, doc_text) in self.docs_lru.into_iter() {
            whole_message.push_str(&format!(
                "[BEGINNNING OF DOCUMENT: {:?}]{}[END OF DOCUMENT: {:?}]",
                url, doc_text, url
            ));
        }

        // I don't feel great about all these loops
        for (url, changes) in self.changes_lru.into_iter() {
            whole_message.push_str(&format!("[BEGINNING OF RECENT CHANGES MADE TO: {:?}]", url));
            let map = ChangesLookup::to_line_map(changes);
            for (line, change_tup_vec) in map.iter() {
                whole_message.push_str(&format!("[BEGINNING OF CHANGES ON LINE {}]", line));
                for tup in change_tup_vec.iter() {
                    whole_message.push_str(&format!("CHAR IDX: {} CHAR: {}", tup.0, tup.1));
                }
                whole_message.push_str(&format!("[END OF CHANGES ON LINE {}]", line));
            }
            whole_message.push_str(&format!("[END OF RECENT CHANGES MADE TO: {:?}]", url));
        }

        Message {
            role,
            content: whole_message,
        }
    }
}
