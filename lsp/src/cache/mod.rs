pub mod lru;
use anyhow::anyhow;
use espionox::agents::memory::{Message, ToMessage};
use lsp_types::{TextDocumentContentChangeEvent, Url};
use once_cell::sync::Lazy;
use std::{
    collections::HashMap,
    sync::{Arc, RwLock},
};

use crate::handle::runes::RuneBufferBurn;

use self::lru::LRUCache;

pub static GLOBAL_CACHE: Lazy<Arc<RwLock<GlobalCache>>> = Lazy::new(GlobalCache::init_ref_counted);

#[derive(Debug, PartialEq, Eq, Hash, Clone)]
struct ChangesLookup {
    pub(super) line: usize,
    pub(super) char_idx: usize,
    pub(super) char: char,
}

#[derive(Debug)]
pub struct GlobalLRU {
    changes: LRUCache<Url, Vec<ChangesLookup>>,
    docs: LRUCache<Url, String>,
    pub should_trigger_listener: Arc<RwLock<bool>>,
    updates_counter: usize,
}

impl Default for GlobalLRU {
    fn default() -> Self {
        GlobalLRU {
            changes: LRUCache::new(5),
            docs: LRUCache::new(5),
            should_trigger_listener: Arc::new(RwLock::new(true)),
            updates_counter: 0,
        }
    }
}

pub type RuneMap = HashMap<char, RuneBufferBurn>;

#[derive(Debug)]
pub struct GlobalCache {
    pub lru: GlobalLRU,
    pub runes: HashMap<Url, RuneMap>,
}

impl GlobalCache {
    pub fn init_ref_counted() -> Arc<RwLock<Self>> {
        Arc::new(RwLock::new(Self {
            lru: GlobalLRU::default(),
            runes: HashMap::new(),
        }))
    }
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

impl GlobalLRU {
    pub fn changes_at_capacity(&self) -> bool {
        self.changes.at_capacity()
    }

    pub fn docs_at_capacity(&self) -> bool {
        self.docs.at_capacity()
    }

    pub fn get_doc(&mut self, url: &Url) -> Option<String> {
        self.docs.get(url)
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
                match self.changes.get(&url) {
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
                        self.changes.update(url.clone(), changes_vec);
                    }
                    None => {
                        self.changes
                            .update(url.clone(), current_line_changes_lookup);
                    }
                }
            }
            self.increment_updates_counter();
            return Ok(());
        }

        Err(anyhow!("No range in change event"))
    }

    pub fn update_doc(&mut self, text: &str, url: Url) {
        self.docs.update(url, text.to_owned());
        self.increment_updates_counter();
    }

    const AMT_CHANGES_TO_TRIGGER_UPDATE: usize = 5;
    fn increment_updates_counter(&mut self) {
        self.updates_counter += 1;
        if self.updates_counter >= Self::AMT_CHANGES_TO_TRIGGER_UPDATE
            && !*self.should_trigger_listener.read().unwrap()
        {
            *self.should_trigger_listener.write().unwrap() = true;
            self.updates_counter = 0;
        }
    }
}

impl ToMessage for GlobalLRU {
    fn to_message(&self, role: espionox::agents::memory::MessageRole) -> Message {
        let mut whole_message = String::from("Here are the most recently accessed documents: ");
        for (url, doc_text) in self.docs.into_iter() {
            whole_message.push_str(&format!(
                "[BEGINNNING OF DOCUMENT: {:?}]{}[END OF DOCUMENT: {:?}]",
                url, doc_text, url
            ));
        }

        // I don't feel great about all these loops
        for (url, changes) in self.changes.into_iter() {
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
