pub mod burns;
pub mod error;
pub mod lru;

use self::{burns::GlobalBurns, error::CacheError, lru::GlobalLRU};

#[derive(Debug)]
pub struct GlobalCache {
    pub lru: GlobalLRU,
    pub burns: GlobalBurns,
}

type CacheResult<T> = Result<T, CacheError>;
impl GlobalCache {
    pub fn init() -> Self {
        Self {
            lru: GlobalLRU::default(),
            burns: GlobalBurns::default(),
        }
    }
}
