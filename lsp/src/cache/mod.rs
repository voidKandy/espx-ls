pub mod error;
pub mod lru;

use std::collections::HashMap;

use crate::burns::{cache::BurnCache, InBufferBurn};

use self::{error::CacheError, lru::GlobalLRU};

#[derive(Debug)]
pub struct GlobalCache {
    pub lru: GlobalLRU,
    pub burns: BurnCache,
    // pub runes: GlobalRunes,
}

type CacheResult<T> = Result<T, CacheError>;
impl GlobalCache {
    pub fn init() -> Self {
        Self {
            lru: GlobalLRU::default(),
            burns: BurnCache::default(),
            // runes: GlobalRunes::default(),
        }
    }
}
