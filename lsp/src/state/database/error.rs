use crate::error::error_chain_fmt;
use espionox::agents::AgentError;
use std::fmt::{Debug, Display, Formatter, Result as FmtResult};

pub type DatabaseResult<T> = Result<T, DatabaseError>;

#[derive(thiserror::Error)]
pub enum DatabaseError {
    #[error(transparent)]
    Undefined(#[from] anyhow::Error),
    Io(#[from] std::io::Error),
    SurrealClient(#[from] surrealdb::Error),
    // AgentError(#[from] AgentError),
}

impl Debug for DatabaseError {
    fn fmt(&self, f: &mut Formatter<'_>) -> FmtResult {
        error_chain_fmt(self, f)
    }
}

impl Display for DatabaseError {
    fn fmt(&self, f: &mut Formatter<'_>) -> FmtResult {
        let display = match self {
            Self::Undefined(err) => err.to_string(),
            Self::Io(err) => err.to_string(),
            Self::SurrealClient(err) => err.to_string(),
            // Self::AgentError(err) => err.to_string(),
        };
        write!(f, "{}", display)
    }
}
