pub mod error;
pub mod lru;
use anyhow::anyhow;
use espionox::agents::memory::{Message, ToMessage};
use log::info;
use lsp_types::{HoverContents, Position, TextDocumentContentChangeEvent, Url};
use std::collections::HashMap;

use crate::handle::runes::RuneBufferBurn;

use self::{error::CacheError, lru::LRUCache};

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
    pub should_trigger_listener: bool,
    updates_counter: usize,
}

impl Default for GlobalLRU {
    fn default() -> Self {
        GlobalLRU {
            changes: LRUCache::new(5),
            docs: LRUCache::new(5),
            should_trigger_listener: true,
            updates_counter: 0,
        }
    }
}

pub type BurnMap = HashMap<String, RuneBufferBurn>;

#[derive(Debug)]
pub struct GlobalCache {
    pub lru: GlobalLRU,
    pub burns: HashMap<Url, BurnMap>,
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

type CacheResult<T> = Result<T, CacheError>;
impl GlobalCache {
    pub fn init() -> Self {
        Self {
            lru: GlobalLRU::default(),
            burns: HashMap::new(),
        }
    }
    pub fn docs_at_capacity(&self) -> bool {
        self.lru.docs.at_capacity()
        // CacheResult::Ok(Self::get_read()?.lru.docs.at_capacity().clone())
    }

    pub fn get_doc(&mut self, url: &Url) -> CacheResult<String> {
        Ok(self.lru.docs.get(url).ok_or(CacheError::NotPresent)?)
    }

    // I don't love these clones
    pub fn get_burn_by_placeholder(&self, url: &Url, rune: &str) -> CacheResult<RuneBufferBurn> {
        Ok(self
            .burns
            .get(url)
            .ok_or(CacheError::NotPresent)?
            .get(rune)
            .ok_or(CacheError::NotPresent)?)
        .cloned()
    }

    pub fn get_hovered_burn(&self, url: &Url, position: Position) -> CacheResult<HoverContents> {
        info!("LOOKING FOR BURN AT POSITION: {:?}", position);
        if let Some(map) = self.burns.get(url) {
            info!("MAP EXISTS: {:?}", map);
            if let Some(found_burn) = map.values().into_iter().find(|burn| {
                let range = burn.range();
                (position.line == range.end.line || position.line == range.start.line)
                    && (position.character >= range.start.character
                        && position.character <= range.end.character)
            }) {
                info!("BURN EXISTS, RETURNING HOVER CONTENTS");
                return Ok(found_burn.hover_contents.clone());
            }
        }
        Err(CacheError::NotPresent)
    }

    pub fn all_burns_on_doc(&self, url: &Url) -> CacheResult<Vec<RuneBufferBurn>> {
        let runes = self.burns.get(url).ok_or(CacheError::NotPresent)?;
        info!("GOT RUNES: {:?}", runes);
        Ok(runes.values().cloned().collect())
    }

    pub fn save_rune(&mut self, url: Url, mut burn: RuneBufferBurn) -> CacheResult<()> {
        match self.burns.get_mut(&url) {
            Some(doc_rune_map) => {
                let already_exist: Vec<&str> = doc_rune_map.keys().map(|k| k.as_str()).collect();
                loop {
                    if !already_exist.contains(&(burn.placeholder.1.as_str())) {
                        break;
                    }
                    burn.placeholder.1 = RuneBufferBurn::generate_placeholder();
                }
                doc_rune_map.insert(burn.placeholder.1.to_owned(), burn);
            }
            None => {
                let mut burns = HashMap::new();
                burns.insert(burn.placeholder.1.to_owned(), burn);
                self.burns.insert(url, burns);
            }
        }

        Ok(())
    }

    pub fn update_doc_changes(
        &mut self,
        event: &TextDocumentContentChangeEvent,
        url: Url,
    ) -> CacheResult<()> {
        info!("UPDATING CACHE FROM TEXTDOCUMENTCHANGEEVENT");
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

                info!("ON LINE: {}", line_number);
                match self.lru.changes.get(&url) {
                    Some(mut changes_vec) => {
                        info!("CHANGES MAP EXISTS, UPDATING..");
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
                        self.lru.changes.update(url.clone(), changes_vec);
                    }
                    None => {
                        info!("CHANGES DOES NOT EXIST, UPDATING..");
                        self.lru
                            .changes
                            .update(url.clone(), current_line_changes_lookup);
                    }
                }
            }
            self.increment_lru_updates_counter()?;
        }

        Err(CacheError::Undefined(anyhow!("No range in change event")))
    }

    pub fn update_doc(&mut self, text: &str, url: Url) -> CacheResult<()> {
        self.lru.docs.update(url, text.to_owned());
        info!("SENT UPDATE TO STATIC CACHE");
        self.increment_lru_updates_counter()
    }

    const AMT_CHANGES_TO_TRIGGER_UPDATE: usize = 5;
    fn increment_lru_updates_counter(&mut self) -> CacheResult<()> {
        info!("UPDATING LRU CHANGES COUNTER");
        let counter: &mut usize = &mut self.lru.updates_counter;
        let mut should_trigger = self.lru.should_trigger_listener;
        *counter += 1;
        if *counter >= Self::AMT_CHANGES_TO_TRIGGER_UPDATE && !should_trigger {
            should_trigger = true;
            *counter = 0;
        }
        Ok(())
    }
}

impl ToMessage for GlobalLRU {
    fn to_message(&self, role: espionox::agents::memory::MessageRole) -> Message {
        let mut whole_message = String::from("Here are the most recently accessed documents: ");
        for (url, doc_text) in self.docs.into_iter() {
            whole_message.push_str(&format!(
                "[BEGINNNING OF DOCUMENT: {}]{}[END OF DOCUMENT: {}]",
                url.as_str(),
                doc_text,
                url.as_str()
            ));
        }

        // I don't feel great about all these loops
        for (url, changes) in self.changes.into_iter() {
            whole_message.push_str(&format!(
                "[BEGINNING OF RECENT CHANGES MADE TO: {}]",
                url.as_str()
            ));
            let map = ChangesLookup::to_line_map(changes);
            for (line, change_tup_vec) in map.iter() {
                whole_message.push_str(&format!("[BEGINNING OF CHANGES ON LINE {}]", line));
                for tup in change_tup_vec.iter() {
                    whole_message.push_str(&format!("CHAR IDX: {} CHAR: {}", tup.0, tup.1));
                }
                whole_message.push_str(&format!("[END OF CHANGES ON LINE {}]", line));
            }
            whole_message.push_str(&format!(
                "[END OF RECENT CHANGES MADE TO: {}]",
                url.as_str()
            ));
        }

        Message {
            role,
            content: whole_message,
        }
    }
}
