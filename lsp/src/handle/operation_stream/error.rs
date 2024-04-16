use anyhow::anyhow;

use crate::error::error_chain_fmt;
use std::fmt::{Debug, Display, Formatter, Result as FmtResult};

pub type BufferOpStreamResult<T> = Result<T, BufferOpStreamError>;

#[derive(thiserror::Error)]
pub enum BufferOpStreamError {
    #[error(transparent)]
    Undefined(#[from] anyhow::Error),
    Send(anyhow::Error),
}

impl<E> From<tokio::sync::mpsc::error::SendError<E>> for BufferOpStreamError {
    fn from(value: tokio::sync::mpsc::error::SendError<E>) -> Self {
        Self::Send(anyhow!("Send Error: {:?}", value))
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
