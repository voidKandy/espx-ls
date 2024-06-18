pub mod burns;
mod database;
pub mod espx;
pub mod store;
use espionox::agents::memory::ToMessage;
use std::sync::Arc;
use store::GlobalStore;
use tokio::sync::{RwLock, RwLockReadGuard, RwLockWriteGuard};
use tracing::warn;

use espx::EspxEnv;

use self::espx::listeners::lru_role;

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
            Ok(g) => {
                warn!("ACQUIRED READ LOCK OF GLOBAL STATE");
                Ok(g)
            }
            Err(e) => {
                warn!("ERROR GETTING READ LOCK: {:?}", e);
                Err(e.into())
            }
        }
    }

    pub fn get_write(&mut self) -> anyhow::Result<RwLockWriteGuard<'_, GlobalState>> {
        match self.0.try_write() {
            Ok(g) => {
                warn!("ACQUIRED WRITE LOCK OF GLOBAL STATE");
                Ok(g)
            }
            Err(e) => {
                warn!("ERROR GETTING WRITE LOCK: {:?}", e);
                Err(e.into())
            }
        }
    }
}

impl GlobalState {
    async fn init() -> anyhow::Result<Self> {
        let store = GlobalStore::init().await;
        let espx_env = EspxEnv::init().await?;
        Ok(Self { store, espx_env })
    }

    /// Uses global state's store to_message method to update the assistant
    pub async fn refresh_update_with_cache(&mut self) -> anyhow::Result<()> {
        let message = self.store.to_message(lru_role());
        let mut wl = self.espx_env.updater.stack_write_lock()?;
        match wl.as_mut() {
            Some(ref mut stack) => {
                stack.mut_filter_by(lru_role(), false);
                stack.push(message);
            }
            None => *wl = Some(vec![message].into()),
        }
        Ok(())
    }
}
