use crate::error::error_chain_fmt;
use std::{
    fmt::{Debug, Display, Formatter, Result as FmtResult},
    io,
    string::FromUtf8Error,
};

pub type StoreResult<T> = Result<T, StoreError>;

#[derive(thiserror::Error)]
pub enum StoreError {
    #[error(transparent)]
    Undefined(#[from] anyhow::Error),
    Utf(#[from] FromUtf8Error),
    Io(#[from] io::Error),
    NotPresent,
}

impl Debug for StoreError {
    fn fmt(&self, f: &mut Formatter<'_>) -> FmtResult {
        error_chain_fmt(self, f)
    }
}

impl Display for StoreError {
    fn fmt(&self, f: &mut Formatter<'_>) -> FmtResult {
        let display = match self {
            Self::Io(err) => err.to_string(),
            Self::Undefined(err) => err.to_string(),
            Self::Utf(err) => err.to_string(),
            // Self::Database(err) => err.to_string(),
            Self::NotPresent => "Not Present".to_string(),
        };
        write!(f, "{}", display)
    }
}
