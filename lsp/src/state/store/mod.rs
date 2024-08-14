mod burns;
mod docs;
pub mod error;
use self::{
    burns::BurnCache,
    docs::DocLRU,
    error::{StoreError, StoreResult},
};
use super::burns::Burn;
use crate::config::Config;
pub use docs::walk_dir;
use espionox::agents::memory::{Message, ToMessage};
use lsp_types::Uri;
use tracing::{debug, warn};

#[derive(Debug)]
pub struct GlobalStore {
    docs: DocLRU,
    pub burns: BurnCache,
}

pub(super) const CACHE_SIZE: usize = 5;

impl ToMessage for GlobalStore {
    fn to_message(&self, role: espionox::agents::memory::MessageRole) -> Message {
        let mut whole_message = String::from("Here are the most recently accessed documents: ");
        for (uri, doc_text) in self.docs.0.into_iter() {
            whole_message.push_str(&format!(
                "[BEGINNNING OF DOCUMENT: {}]\n{}\n[END OF DOCUMENT: {}]\n",
                uri.as_str(),
                doc_text,
                uri.as_str()
            ));
        }
        debug!("LRU CACHE COERCED TO MESSAGE: {}", whole_message);

        Message {
            role,
            content: whole_message,
        }
    }
}

impl GlobalStore {
    #[tracing::instrument(name = "building store from config")]
    pub async fn from_config(cfg: &Config) -> Self {
        debug!("success building global store");
        Self {
            docs: DocLRU::default(),
            burns: BurnCache::default(),
        }
    }

    pub fn docs_at_capacity(&self) -> bool {
        self.docs.0.at_capacity()
    }

    pub fn all_docs(&self) -> Vec<(&Uri, &str)> {
        self.docs
            .0
            .read_all()
            .into_iter()
            .map(|(k, v)| (k, v.as_str()))
            .collect()
    }

    /// This should be used very sparingly as it completely circumvents the utility of an LRU
    pub fn read_doc(&self, uri: &Uri) -> StoreResult<String> {
        self.docs
            .0
            .read(uri)
            .ok_or(StoreError::new_not_present(uri.as_str()))
    }

    pub fn get_doc(&mut self, uri: &Uri) -> StoreResult<String> {
        self.docs
            .0
            .get(uri)
            .ok_or(StoreError::new_not_present(uri.as_str()))
    }

    // Maybe a function could be written to see if two burns are likely the same
    pub fn update_burns_on_doc(&mut self, uri: &Uri) -> StoreResult<()> {
        let text = self.read_doc(uri)?;
        let mut prev_burns_opt = self.burns.take_burns_on_doc(uri);
        for new_burn in Burn::all_in_text(&text) {
            match prev_burns_opt.as_mut().and_then(|bv| {
                bv.iter()
                    .position(|b| {
                        b.activation.matches_variant(&new_burn.activation) && {
                            let range = b.activation.range();
                            match range.peek_left() {
                                Some(range) => new_burn.activation.overlaps(range.as_ref()),
                                None => {
                                    let ranges = range.peek_right().unwrap();
                                    new_burn.activation.overlaps(ranges.0.as_ref())
                                        || new_burn.activation.overlaps(ranges.1.as_ref())
                                }
                            }
                        }
                    })
                    .and_then(|i| Some(bv.remove(i)))
            }) {
                Some(b) => self.burns.insert_burn(uri.clone(), b),
                None => self.burns.insert_burn(uri.clone(), new_burn),
            }
        }
        Ok(())
    }
    #[tracing::instrument(name = "updating document", skip_all)]
    pub fn update_doc(&mut self, text: &str, uri: Uri) {
        self.docs.0.update(uri, text.to_owned());
    }
}
