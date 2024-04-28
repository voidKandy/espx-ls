use crate::{error::error_chain_fmt, store};
use std::fmt::{Debug, Display, Formatter, Result as FmtResult};

#[derive(thiserror::Error)]
pub enum DiagnosticError {
    #[error(transparent)]
    Undefined(#[from] anyhow::Error),
    Store(#[from] store::error::StoreError),
}

impl Debug for DiagnosticError {
    fn fmt(&self, f: &mut Formatter<'_>) -> FmtResult {
        error_chain_fmt(self, f)
    }
}

impl Display for DiagnosticError {
    fn fmt(&self, f: &mut Formatter<'_>) -> FmtResult {
        write!(f, "{:?}", self)
    }
}
