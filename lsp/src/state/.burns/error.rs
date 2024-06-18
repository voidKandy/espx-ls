use espionox::agents::AgentError;

use crate::{
    error::error_chain_fmt, handle::buffer_operations::BufferOpError,
    state::store::error::StoreError,
};
use std::fmt::{Debug, Display, Formatter, Result as FmtResult};

pub type BurnResult<T> = Result<T, BurnError>;

#[derive(thiserror::Error)]
pub enum BurnError {
    #[error(transparent)]
    Undefined(#[from] anyhow::Error),
    Json(#[from] serde_json::Error),
    Store(#[from] StoreError),
    BufferOp(#[from] BufferOpError),
    Agent(#[from] AgentError),
    // Send,
    ActionType,
    EchoType,
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
