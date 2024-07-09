mod burns;
mod docs;
pub mod error;
mod lru;
mod tests;
use self::{
    burns::BurnCache,
    docs::DocLRU,
    error::{StoreError, StoreResult},
};
use super::{
    burns::{Burn, BurnActivation, MultiLineBurn, SingleLineBurn},
    database::{docs::FullDBDocument, Database},
};
use crate::{config::GLOBAL_CONFIG, parsing};
pub use docs::{update_text_with_change, walk_dir};
use espionox::agents::memory::{Message, ToMessage};
use lsp_types::{TextDocumentContentChangeEvent, Uri};
use tracing::debug;

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
impl DatabaseStore {
    pub async fn read_all_docs_to_cache(&mut self) -> anyhow::Result<()> {
        let docs = FullDBDocument::get_all_docs(&self.client).await?;
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

    pub fn update_doc_from_lsp_change_notification(
        &mut self,
        change: &TextDocumentContentChangeEvent,
        uri: Uri,
    ) -> StoreResult<()> {
        let text = self.get_doc(&uri)?;
        let new_text = update_text_with_change(&text, change)?;
        self.docs.0.update(uri, new_text);
        Ok(())
    }

    pub fn update_burns_on_doc(&mut self, uri: &Uri) -> StoreResult<()> {
        let text = self.get_doc(&uri)?;

        for burn in SingleLineBurn::all_variants() {
            let mut lines = parsing::all_lines_with_pattern(&burn.trigger_string(), &text);
            lines.append(&mut parsing::all_lines_with_pattern(
                &burn.echo_content(),
                &text,
            ));
            for l in lines {
                // let mut diags = ::burn_diagnostics_on_line(&burn, l, &text)?;
                // all_diagnostics.append(&mut diags);
                self.burns
                    .insert_burn(uri.clone(), l, BurnActivation::Single(burn.clone()));
            }
        }
        for burn in MultiLineBurn::all_variants() {
            let lines_and_chars =
                parsing::all_lines_with_pattern_with_char_positions(&burn.trigger_string(), &text);
            for (l, _) in lines_and_chars {
                self.burns
                    .insert_burn(uri.clone(), l, BurnActivation::Multi(burn.clone()));
            }
        }
        Ok(())
    }

    #[tracing::instrument(name = "updating document", skip(self))]
    pub fn update_doc(&mut self, text: &str, uri: Uri) {
        self.docs.0.update(uri, text.to_owned());
    }
}
