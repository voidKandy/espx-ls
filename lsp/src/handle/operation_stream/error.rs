use anyhow::anyhow;
use crossbeam_channel::SendError;
use lsp_server::Message;

use crate::{
    error::error_chain_fmt,
    handle::diagnostics::error::DiagnosticError,
    store::{database::error::DBModelError, error::StoreError},
};
use std::fmt::{Debug, Display, Formatter, Result as FmtResult};

pub type BufferOpStreamResult<T> = Result<T, BufferOpStreamError>;

#[derive(thiserror::Error)]
pub enum BufferOpStreamError {
    #[error(transparent)]
    Undefined(#[from] anyhow::Error),
    TokioSend(anyhow::Error),
    CrossBeamSend(#[from] SendError<Message>),
    Json(#[from] serde_json::Error),
    Store(#[from] StoreError),
    Diagnostic(#[from] DiagnosticError),
}

impl<E> From<tokio::sync::mpsc::error::SendError<E>> for BufferOpStreamError {
    fn from(value: tokio::sync::mpsc::error::SendError<E>) -> Self {
        Self::TokioSend(anyhow!("Send Error: {:?}", value))
    }
}

impl From<DBModelError> for BufferOpStreamError {
    fn from(value: DBModelError) -> Self {
        Self::Store(StoreError::from(value))
    }
}

impl Debug for BufferOpStreamError {
    fn fmt(&self, f: &mut Formatter<'_>) -> FmtResult {
        error_chain_fmt(self, f)
    }
}

impl Display for BufferOpStreamError {
    fn fmt(&self, f: &mut Formatter<'_>) -> FmtResult {
        write!(f, "{:?}", self)
    }
}
