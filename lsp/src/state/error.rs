use crate::{error::error_chain_fmt, state::database::error::DatabaseError};
use std::{
    fmt::{Debug, Display, Formatter, Result as FmtResult},
    io,
    string::FromUtf8Error,
};

use super::{burns::error::BurnError, store::error::StoreError};

pub type StateResult<T> = Result<T, StateError>;

#[derive(thiserror::Error)]
pub enum StateError {
    #[error(transparent)]
    Undefined(#[from] anyhow::Error),
    // Database(#[from] DatabaseError),
    Store(#[from] StoreError),
    Burn(#[from] BurnError),
}

impl From<DatabaseError> for StateError {
    fn from(value: DatabaseError) -> Self {
        Self::Store(value.into())
    }
}

impl Debug for StateError {
    fn fmt(&self, f: &mut Formatter<'_>) -> FmtResult {
        error_chain_fmt(self, f)
    }
}

impl Display for StateError {
    fn fmt(&self, f: &mut Formatter<'_>) -> FmtResult {
        let display = match self {
            Self::Burn(err) => err.to_string(),
            Self::Undefined(err) => err.to_string(),
            Self::Store(err) => err.to_string(),
        };
        write!(f, "{}", display)
    }
}
