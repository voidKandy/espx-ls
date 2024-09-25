use espionox::agents::AgentError;

use crate::{error::error_chain_fmt, handle::buffer_operations::BufferOpError};
use std::fmt::{Debug, Display, Formatter, Result as FmtResult};

pub type BurnResult<T> = Result<T, BurnError>;

#[derive(thiserror::Error)]
pub enum BurnError {
    #[error(transparent)]
    Undefined(#[from] anyhow::Error),
    // Agent(#[from] AgentError),
    WrongVariant,
}

impl Debug for BurnError {
    fn fmt(&self, f: &mut Formatter<'_>) -> FmtResult {
        error_chain_fmt(self, f)
    }
}

impl Display for BurnError {
    fn fmt(&self, f: &mut Formatter<'_>) -> FmtResult {
        let display = match self {
            Self::Undefined(err) => err.to_string(),
            // Self::BuferOp(err) => err.to_string(),
            // Self::Agent(err) => err.to_string(),
            Self::WrongVariant => "Wrong variant".to_string(),
        };
        write!(f, "{}", display)
    }
}
