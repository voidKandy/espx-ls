pub mod burns;
pub mod database;
pub mod error;
pub mod espx;
pub mod store;
use anyhow::anyhow;
use database::docs::chunks::ChunkVector;
use espionox::{
    agents::{memory::ToMessage, Agent},
    prelude::MessageRole,
};
use futures::Future;
use lsp_types::Uri;
use std::sync::Arc;
use store::GlobalStore;
use tokio::sync::{RwLock, RwLockReadGuard, RwLockWriteGuard};
use tracing::{debug, warn};

use espx::{listeners::rag::lru_role, AgentID, EspxEnv};

use crate::config::GLOBAL_CONFIG;

use self::{
    database::docs::{chunks::DBDocumentChunk, info::DBDocumentInfo, FullDBDocument},
    error::StateResult,
    store::{error::StoreError, DatabaseStore},
};

#[derive(Debug)]
pub struct GlobalState {
    pub store: GlobalStore,
    pub espx_env: EspxEnv,
}

#[derive(Debug)]
pub struct SharedGlobalState(Arc<RwLock<GlobalState>>);

impl SharedGlobalState {
    pub async fn init() -> anyhow::Result<Self> {
        Ok(Self(Arc::new(RwLock::new(GlobalState::init().await?))))
    }
}

impl Clone for SharedGlobalState {
    fn clone(&self) -> Self {
        Self(Arc::clone(&self.0))
    }
}

impl SharedGlobalState {
    pub fn get_read(&self) -> anyhow::Result<RwLockReadGuard<'_, GlobalState>> {
        match self.0.try_read() {
            Ok(g) => Ok(g),
            Err(e) => Err(e.into()),
        }
    }

    pub fn get_write(&mut self) -> anyhow::Result<RwLockWriteGuard<'_, GlobalState>> {
        match self.0.try_write() {
            Ok(g) => Ok(g),
            Err(e) => Err(e.into()),
        }
    }
}

impl GlobalState {
    async fn init() -> StateResult<Self> {
        let store = GlobalStore::init().await;
        let espx_env = EspxEnv::init().await?;
        Ok(Self { store, espx_env })
    }

    /// Uses global state's store to_message method to update the assistant
    pub async fn refresh_update_with_cache(&mut self) -> StateResult<()> {
        let message = self.store.to_message(lru_role());
        let mut wl = self.espx_env.updater.stack_write_lock()?;
        match wl.as_mut() {
            Some(ref mut stack) => {
                stack.mut_filter_by(&lru_role(), false);
                stack.push(message);
            }
            None => *wl = Some(vec![message].into()),
        }
        Ok(())
    }

    pub async fn update_doc_store(&mut self, text: &str, uri: Uri) -> StateResult<()> {
        let db: &DatabaseStore = self.store.db.as_ref().ok_or(StoreError::new_not_present(
            "store has no database connection",
        ))?;
        match FullDBDocument::get_by_uri(&db.client, &uri).await? {
            None => {
                let doc = FullDBDocument::from_state(&self.store, uri.clone())
                    .await
                    .expect("Failed to build dbdoc tuple");
                let _ = &doc.info.insert(&db.client).await?;
                let _ = &doc.chunks.insert(&db.client).await?;
            }
            Some(doc) => {
                if &doc.chunks.into_text() != text {
                    ChunkVector::remove_multiple_by_uri(&db.client, &uri)
                        .await
                        .expect("Could not remove chunks");
                    let chunks = ChunkVector::from_text(uri.clone(), &text)?;
                    let _ = chunks.insert(&db.client).await?;
                }
            }
        }
        Ok(())
    }

    pub fn update_conversation_file(&mut self, agent: &Agent) -> StateResult<()> {
        let mut out_string_vec = vec![];
        for message in agent.cache.as_ref().into_iter() {
            let role_str = {
                if let MessageRole::Other { alias, .. } = &message.role {
                    alias.to_string()
                } else {
                    message.role.to_string()
                }
            };
            let role_str = convert_ascii(&role_str, '𝐀');
            out_string_vec.push(format!("# {}\n\n", &role_str));

            for chunk in message.content.split(". ") {
                out_string_vec.push(chunk.to_owned());
            }
        }
        let content_to_write = out_string_vec.join("\n");
        warn!("updating conversation file: {}", content_to_write);
        std::fs::write(
            GLOBAL_CONFIG.paths.conversation_file_path.clone(),
            content_to_write,
        )
        .unwrap();
        return Ok(());
    }
}

// For making the role look 𝐍 𝐈 𝐂 𝐄
fn convert_ascii(str: &str, target: char) -> String {
    let start_code_point = target as u32;
    let str = str.to_lowercase();
    let mut chars = vec![' '];
    str.chars().for_each(|c| {
        let offset = c as u32 - 'a' as u32;
        chars.push(std::char::from_u32(start_code_point + offset).unwrap_or(c));
        chars.push(' ');
    });

    chars.into_iter().collect()
}
