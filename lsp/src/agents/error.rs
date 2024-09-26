use crate::error::error_chain_fmt;
use lsp_types::Uri;
use std::fmt::{Debug, Display, Formatter, Result as FmtResult};

pub type AgentsResult<T> = Result<T, AgentsError>;

#[derive(thiserror::Error)]
pub enum AgentsError {
    #[error(transparent)]
    Undefined(#[from] anyhow::Error),
    DocAgentNotPresent(Uri),
}

impl Debug for AgentsError {
    fn fmt(&self, f: &mut Formatter<'_>) -> FmtResult {
        error_chain_fmt(self, f)
    }
}

impl Display for AgentsError {
    fn fmt(&self, f: &mut Formatter<'_>) -> FmtResult {
        let display = match self {
            Self::Undefined(err) => err.to_string(),
            Self::DocAgentNotPresent(uri) => {
                format!("No agent present for document: {}", uri.to_string())
            }
        };
        write!(f, "{}", display)
    }
}
