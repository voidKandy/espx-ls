pub mod burns;
pub mod db_integration;
pub mod error;
pub mod lru;
pub mod tests;
use self::{burns::BurnCache, lru::GlobalLRU};

#[derive(Debug)]
pub struct GlobalCache {
    pub lru: GlobalLRU,
    pub burns: BurnCache,
}

impl GlobalCache {
    pub fn init() -> Self {
        Self {
            lru: GlobalLRU::default(),
            burns: BurnCache::default(),
        }
    }
}
