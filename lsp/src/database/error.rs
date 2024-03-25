use std::fmt::{Debug, Display, Formatter, Result as FmtResult};

use espionox::agents::AgentError;

use crate::espx_env::agents::independent::IndyAgent;

pub fn error_chain_fmt(
    e: &impl std::error::Error,
    f: &mut std::fmt::Formatter<'_>,
) -> std::fmt::Result {
    writeln!(f, "{}\n", e);
    let mut current = e.source();
    while let Some(cause) = current {
        writeln!(f, "Caused by:\n\t{}", cause)?;
        current = cause.source();
    }
    Ok(())
}

#[derive(thiserror::Error)]
pub enum DbModelError {
    #[error(transparent)]
    Undefined(#[from] anyhow::Error),
    FailedToGetAgent(IndyAgent),
    IndyAgentError(#[from] AgentError),
}

impl Debug for DbModelError {
    fn fmt(&self, f: &mut Formatter<'_>) -> FmtResult {
        error_chain_fmt(self, f)
    }
}

impl Display for DbModelError {
    fn fmt(&self, f: &mut Formatter<'_>) -> FmtResult {
        write!(f, "{:?}", self)
    }
}
