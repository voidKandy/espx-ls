use crate::{database::error::DbModelError, error::error_chain_fmt};
use std::fmt::{Debug, Display, Formatter, Result as FmtResult};

pub type CacheResult<T> = Result<T, CacheError>;

#[derive(thiserror::Error)]
pub enum CacheError {
    #[error(transparent)]
    Undefined(#[from] anyhow::Error),
    Database(#[from] DbModelError),
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
