use crate::error::error_chain_fmt;
use std::fmt::{Debug, Display, Formatter, Result as FmtResult};

#[derive(thiserror::Error)]
pub enum CacheError {
    #[error(transparent)]
    Undefined(#[from] anyhow::Error),
    NotPresent,
    NoStaticInst,
}

impl Debug for CacheError {
    fn fmt(&self, f: &mut Formatter<'_>) -> FmtResult {
        error_chain_fmt(self, f)
    }
}

impl Display for CacheError {
    fn fmt(&self, f: &mut Formatter<'_>) -> FmtResult {
        write!(f, "{:?}", self)
    }
}
