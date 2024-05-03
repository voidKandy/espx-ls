use espionox::environment::EnvHandleError;

use crate::{
    error::error_chain_fmt,
    handle::operation_stream::BufferOpStreamError,
    store::{database::error::DBModelError, error::StoreError},
};
use std::fmt::{Debug, Display, Formatter, Result as FmtResult};

pub type BurnResult<T> = Result<T, BurnError>;

#[derive(thiserror::Error)]
pub enum BurnError {
    #[error(transparent)]
    Undefined(#[from] anyhow::Error),
    Json(#[from] serde_json::Error),
    EspxEnv(#[from] EnvHandleError),
    Store(#[from] StoreError),
    BufferOpStream(#[from] BufferOpStreamError),
    Send,
    ActionType,
    EchoType,
}

impl<E> From<crossbeam_channel::SendError<E>> for BurnError {
    fn from(_: crossbeam_channel::SendError<E>) -> Self {
        Self::Send
    }
}

impl From<DBModelError> for BurnError {
    fn from(value: DBModelError) -> Self {
        Self::Store(value.into())
    }
}

impl Debug for BurnError {
    fn fmt(&self, f: &mut Formatter<'_>) -> FmtResult {
        error_chain_fmt(self, f)
    }
}

impl Display for BurnError {
    fn fmt(&self, f: &mut Formatter<'_>) -> FmtResult {
        write!(f, "{:?}", self)
    }
}
