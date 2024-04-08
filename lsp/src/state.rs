use std::sync::Arc;

use log::warn;
use tokio::sync::{RwLock, RwLockReadGuard, RwLockWriteGuard};

use crate::cache::GlobalCache;

#[derive(Debug)]
pub struct GlobalState {
    pub cache: GlobalCache,
}

#[derive(Debug)]
pub struct SharedGlobalState(Arc<RwLock<GlobalState>>);

impl Default for SharedGlobalState {
    fn default() -> Self {
        Self(Arc::new(RwLock::new(GlobalState::default())))
    }
}

impl Default for GlobalState {
    fn default() -> Self {
        let cache = GlobalCache::init();
        Self { cache }
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
