pub mod burns;
pub mod db_integration;
pub mod error;
pub mod lru;
use self::{burns::BurnCache, error::CacheError, lru::GlobalLRU};

#[derive(Debug)]
pub struct GlobalCache {
    pub lru: GlobalLRU,
    pub burns: BurnCache,
}

type CacheResult<T> = Result<T, CacheError>;
impl GlobalCache {
    pub fn init() -> Self {
        Self {
            lru: GlobalLRU::default(),
            burns: BurnCache::default(),
        }
    }
}
