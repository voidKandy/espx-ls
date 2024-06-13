use super::buffer_operations::{BufferOpChannelError, BufferOpError};
use crate::{
    error::error_chain_fmt,
    state::{burns::error::BurnError, store::error::StoreError},
};
use std::fmt::{Debug, Display, Formatter, Result as FmtResult};

pub type HandleResult<T> = Result<T, HandleError>;
#[derive(thiserror::Error)]
pub enum HandleError {
    #[error(transparent)]
    Undefined(#[from] anyhow::Error),
    Json(#[from] serde_json::error::Error),
    BufferOp(#[from] BufferOpError),
    Store(#[from] StoreError),
    Burn(#[from] BurnError),
}

impl Debug for HandleError {
    fn fmt(&self, f: &mut Formatter<'_>) -> FmtResult {
        error_chain_fmt(self, f)
    }
}

impl Display for HandleError {
    fn fmt(&self, f: &mut Formatter<'_>) -> FmtResult {
        let display = match self {
            Self::Undefined(err) => err.to_string(),
            Self::Burn(err) => err.to_string(),
            Self::BufferOp(err) => err.to_string(),
            Self::Json(err) => err.to_string(),
            Self::Store(err) => err.to_string(),
        };
        write!(f, "{}", display)
    }
}

impl From<BufferOpChannelError> for HandleError {
    fn from(value: BufferOpChannelError) -> Self {
        Self::BufferOp(Into::<BufferOpError>::into(value))
    }
}
