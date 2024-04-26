pub mod util;
use espionox::agents::memory::{Message, ToMessage};
use log::{debug, info};
use lsp_types::Url;
use std::{
    collections::HashMap,
    sync::{Arc, RwLock},
};

use crate::espx_env::listeners::LRURAG;
use util::LRUCache;

use super::error::{CacheError, CacheResult};

#[derive(Debug, PartialEq, Eq, Hash, Clone)]
struct ChangesLookup {
    pub(super) line: usize,
    pub(super) char_idx: usize,
    pub(super) char: char,
}

#[derive(Debug)]
pub struct GlobalLRU {
    ///  When Some(), listener is triggered and assistant is given up to date context
    pub listener_update: Arc<RwLock<Option<Message>>>,
    /// When hits AMT, above field is made Some()
    updates_counter: usize,

    pub(super) docs: LRUCache<Url, String>,
}
const AMT_CHANGES_TO_TRIGGER_UPDATE: usize = 5;

impl Default for GlobalLRU {
    fn default() -> Self {
        GlobalLRU {
            // changes: LRUCache::new(5),
            docs: LRUCache::new(5),
            listener_update: Arc::new(RwLock::new(None)),
            updates_counter: 0,
        }
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
    pub fn docs_at_capacity(&self) -> bool {
        self.docs.at_capacity()
        // CacheResult::Ok(Self::get_read()?.lru.docs.at_capacity().clone())
    }

    pub fn get_doc(&mut self, url: &Url) -> CacheResult<String> {
        Ok(self.docs.get(url).ok_or(CacheError::NotPresent)?)
    }

    pub fn tell_listener_to_update_agent(&mut self) -> CacheResult<()> {
        *self
            .listener_update
            .write()
            .expect("Couldn't write lock listener_update") = Some(self.to_message(LRURAG::role()));
        Ok(())
    }

    // pub fn update_doc_changes(
    //     &mut self,
    //     event: &TextDocumentContentChangeEvent,
    //     url: Url,
    // ) -> CacheResult<()> {
    //     info!("UPDATING CACHE FROM TEXTDOCUMENTCHANGEEVENT");
    //     if let Some(range) = event.range {
    //         let texts: Vec<&str> = event.text.lines().collect();
    //         let start_char = range.start.character as usize;
    //         let start_line = range.start.line as usize;
    //
    //         for (line_number, t) in texts.into_iter().enumerate() {
    //             let current_line = start_line + line_number;
    //             let mut current_line_changes_lookup = Vec::new();
    //             t.char_indices().for_each(|(char_idx, char)| {
    //                 current_line_changes_lookup.push(ChangesLookup {
    //                     line: current_line,
    //                     char_idx: char_idx + start_char,
    //                     char,
    //                 })
    //             });
    //
    //             info!("ON LINE: {}", line_number);
    //             match self.changes.get(&url) {
    //                 Some(mut changes_vec) => {
    //                     info!("CHANGES MAP EXISTS, UPDATING..");
    //                     changes_vec.iter_mut().for_each(|change| {
    //                         if let Some(idx) =
    //                             current_line_changes_lookup.iter_mut().position(|ch| {
    //                                 ch.line == change.line && ch.char_idx == change.char_idx
    //                             })
    //                         {
    //                             let overwrite = current_line_changes_lookup.swap_remove(idx);
    //                             change.char = overwrite.char;
    //                         }
    //                     });
    //                     if !current_line_changes_lookup.is_empty() {
    //                         changes_vec.append(&mut current_line_changes_lookup);
    //                     }
    //                     self.changes.update(url.clone(), changes_vec);
    //                 }
    //                 None => {
    //                     info!("CHANGES DOES NOT EXIST, UPDATING..");
    //                     self.changes
    //                         .update(url.clone(), current_line_changes_lookup);
    //                 }
    //             }
    //         }
    //         self.increment_lru_updates_counter()?;
    //     }
    //
    //     Err(CacheError::Undefined(anyhow!("No range in change event")))
    // }

    pub fn update_doc(&mut self, text: &str, url: Url) -> CacheResult<()> {
        self.docs.update(url, text.to_owned());
        info!("SENT UPDATE TO STATIC CACHE");
        self.increment_lru_updates_counter()
    }

    fn increment_lru_updates_counter(&mut self) -> CacheResult<()> {
        info!("UPDATING LRU CHANGES COUNTER");
        let should_trigger = self.listener_update.read().unwrap().is_some();
        self.updates_counter += 1;
        if self.updates_counter >= AMT_CHANGES_TO_TRIGGER_UPDATE && !should_trigger {
            self.tell_listener_to_update_agent()?;
            self.updates_counter = 0;
        }
        Ok(())
    }
}

impl ToMessage for GlobalLRU {
    fn to_message(&self, role: espionox::agents::memory::MessageRole) -> Message {
        let mut whole_message = String::from("Here are the most recently accessed documents: ");
        for (url, doc_text) in self.docs.into_iter() {
            whole_message.push_str(&format!(
                "[BEGINNNING OF DOCUMENT: {}]\n{}\n[END OF DOCUMENT: {}]\n",
                url.as_str(),
                doc_text,
                url.as_str()
            ));
        }

        // I don't feel great about all these loops
        // for (url, changes) in self.changes.into_iter() {
        //     whole_message.push_str(&format!(
        //         "[BEGINNING OF RECENT CHANGES MADE TO: {}]",
        //         url.as_str()
        //     ));
        //     let map = ChangesLookup::to_line_map(changes);
        //     for (line, change_tup_vec) in map.iter() {
        //         whole_message.push_str(&format!("[BEGINNING OF CHANGES ON LINE {}]", line));
        //         for tup in change_tup_vec.iter() {
        //             whole_message.push_str(&format!("CHAR IDX: {} CHAR: {}", tup.0, tup.1));
        //         }
        //         whole_message.push_str(&format!("[END OF CHANGES ON LINE {}]", line));
        //     }
        //     whole_message.push_str(&format!(
        //         "[END OF RECENT CHANGES MADE TO: {}]",
        //         url.as_str()
        //     ));
        // }

        debug!("LRU CACHE COERCED TO MESSAGE: {}", whole_message);

        Message {
            role,
            content: whole_message,
        }
    }
}
