mod burns;
pub mod database;
pub mod error;
mod tests;
mod updater;
use crate::{espx_env::listeners::LRURAG, util::LRUCache};
use burns::BurnCache;
use espionox::agents::memory::{Message, ToMessage};
use log::{debug, info};
use lsp_types::Url;
use updater::AssistantUpdater;

#[derive(Debug, Default)]
pub struct GlobalStore {
    pub docs: DocLRU,
    pub burns: BurnCache,
    pub updater: AssistantUpdater,
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

impl GlobalStore {
    pub fn docs_at_capacity(&self) -> bool {
        self.docs.0.at_capacity()
        // CacheResult::Ok(Self::get_read()?.lru.docs.at_capacity().clone())
    }

    pub fn get_doc(&mut self, url: &Url) -> Option<String> {
        self.docs.0.get(url)
    }

    pub fn tell_listener_to_update_agent(&mut self) {
        *self
            .updater
            .update_message
            .write()
            .expect("Couldn't write lock listener_update") = Some(self.to_message(LRURAG::role()));
    }

    pub fn update_doc(&mut self, text: &str, url: Url) {
        self.docs.0.update(url, text.to_owned());
        info!("SENT UPDATE TO STATIC CACHE");
        self.increment_lru_updates_counter()
    }

    fn increment_lru_updates_counter(&mut self) {
        info!("UPDATING LRU CHANGES COUNTER");
        let should_trigger = self.updater.update_message.read().unwrap().is_some();
        self.updater.counter += 1;
        if self.updater.counter >= updater::AMT_CHANGES_TO_TRIGGER_UPDATE && !should_trigger {
            self.tell_listener_to_update_agent();
            self.updater.counter = 0;
        }
    }
}
