use crate::{cache::error::CacheError, error::error_chain_fmt};
use std::fmt::{Debug, Display, Formatter, Result as FmtResult};

use super::{actions::error::ActionError, diagnostics::error::DiagnosticError};

#[derive(thiserror::Error)]
pub enum EspxLsHandleError {
    #[error(transparent)]
    Undefined(#[from] anyhow::Error),
    Io(#[from] std::io::Error),
    Cache(#[from] CacheError),
    Diagnostic(#[from] DiagnosticError),
    Action(#[from] ActionError),
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
