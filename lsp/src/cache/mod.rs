pub mod burns;
pub mod error;
pub mod lru;

use self::{burns::GlobalRunes, error::CacheError, lru::GlobalLRU};

#[derive(Debug)]
pub struct GlobalCache {
    pub lru: GlobalLRU,
    pub runes: GlobalRunes,
}

type CacheResult<T> = Result<T, CacheError>;
impl GlobalCache {
    pub fn init() -> Self {
        Self {
            lru: GlobalLRU::default(),
            runes: GlobalRunes::default(),
        }
    }
}
