use espionox::environment::EnvHandleError;

use crate::error::error_chain_fmt;
use std::fmt::{Debug, Display, Formatter, Result as FmtResult};

#[derive(thiserror::Error)]
pub enum ActionError {
    #[error(transparent)]
    Undefined(#[from] anyhow::Error),
    Json(#[from] serde_json::Error),
    EspxEnv(#[from] EnvHandleError),
    Send,
    UnimplementedMethod,
}

impl<E> From<crossbeam_channel::SendError<E>> for ActionError {
    fn from(_: crossbeam_channel::SendError<E>) -> Self {
        Self::Send
    }
}

impl Debug for ActionError {
    fn fmt(&self, f: &mut Formatter<'_>) -> FmtResult {
        error_chain_fmt(self, f)
    }
}

impl Display for ActionError {
    fn fmt(&self, f: &mut Formatter<'_>) -> FmtResult {
        write!(f, "{:?}", self)
    }
}
