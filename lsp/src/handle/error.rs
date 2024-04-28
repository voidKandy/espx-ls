use crate::{error::error_chain_fmt, store::error::StoreError};
use std::fmt::{Debug, Display, Formatter, Result as FmtResult};

use super::{diagnostics::error::DiagnosticError, operation_stream::error::BufferOpStreamError};

#[derive(thiserror::Error)]
pub enum EspxLsHandleError {
    #[error(transparent)]
    Undefined(#[from] anyhow::Error),
    Io(#[from] std::io::Error),
    Stream(#[from] BufferOpStreamError),
    Store(#[from] StoreError),
    Diagnostic(#[from] DiagnosticError),
    // Action(#[from] ActionError),
    Json(#[from] serde_json::Error),
}

impl Debug for EspxLsHandleError {
    fn fmt(&self, f: &mut Formatter<'_>) -> FmtResult {
        error_chain_fmt(self, f)
    }
}

impl Display for EspxLsHandleError {
    fn fmt(&self, f: &mut Formatter<'_>) -> FmtResult {
        write!(f, "{:?}", self)
    }
}
