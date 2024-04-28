use super::database::error::DBModelError;
use crate::error::error_chain_fmt;
use std::fmt::{Debug, Display, Formatter, Result as FmtResult};

pub type StoreResult<T> = Result<T, StoreError>;

#[derive(thiserror::Error)]
pub enum StoreError {
    #[error(transparent)]
    Undefined(#[from] anyhow::Error),
    Database(#[from] DBModelError),
    NotPresent,
    NoStaticInst,
}

impl Debug for StoreError {
    fn fmt(&self, f: &mut Formatter<'_>) -> FmtResult {
        error_chain_fmt(self, f)
    }
}

impl Display for StoreError {
    fn fmt(&self, f: &mut Formatter<'_>) -> FmtResult {
        write!(f, "{:?}", self)
    }
}
