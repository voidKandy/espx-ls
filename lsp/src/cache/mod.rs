pub mod error;
pub mod lru;
pub mod runes;

use self::{error::CacheError, lru::GlobalLRU, runes::GlobalRunes};

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
