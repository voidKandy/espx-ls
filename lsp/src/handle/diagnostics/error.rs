use crate::{cache, error::error_chain_fmt, handle::runes};
use std::fmt::{Debug, Display, Formatter, Result as FmtResult};

#[derive(thiserror::Error)]
pub enum DiagnosticError {
    #[error(transparent)]
    Undefined(#[from] anyhow::Error),
    Rune(#[from] runes::error::RuneError),
    Cache(#[from] cache::error::CacheError),
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
