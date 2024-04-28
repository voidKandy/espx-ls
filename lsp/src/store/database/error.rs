use crate::error::error_chain_fmt;
use crate::espx_env::agents::independent::IndyAgent;
use espionox::agents::AgentError;
use std::fmt::{Debug, Display, Formatter, Result as FmtResult};

pub type DBModelResult<T> = Result<T, DBModelError>;

#[derive(thiserror::Error)]
pub enum DBModelError {
    #[error(transparent)]
    Undefined(#[from] anyhow::Error),
    Io(#[from] std::io::Error),
    SurrealClient(#[from] surrealdb::Error),
    FailedToGetAgent(IndyAgent),
    IndyAgentError(#[from] AgentError),
}

impl Debug for DBModelError {
    fn fmt(&self, f: &mut Formatter<'_>) -> FmtResult {
        error_chain_fmt(self, f)
    }
}

impl Display for DBModelError {
    fn fmt(&self, f: &mut Formatter<'_>) -> FmtResult {
        write!(f, "{:?}", self)
    }
}
