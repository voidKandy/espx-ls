use crate::{espx_env::EspxEnv, store::GlobalStore};
use log::warn;
use std::sync::Arc;
use tokio::sync::{RwLock, RwLockReadGuard, RwLockWriteGuard};

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

impl GlobalState {
    async fn init() -> anyhow::Result<Self> {
        let store = GlobalStore::init().await;
        let espx_env = EspxEnv::init(&store).await?;

        Ok(Self { store, espx_env })
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
