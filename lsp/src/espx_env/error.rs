use crate::error::error_chain_fmt;
use std::fmt::{Debug, Display, Formatter, Result as FmtResult};

pub type EspxEnvResult<T> = Result<T, EspxEnvError>;

#[derive(thiserror::Error)]
pub enum EspxEnvError {
    #[error(transparent)]
    Undefined(#[from] anyhow::Error),
    Environment(#[from] espionox::environment::EnvError),
    Handle(#[from] espionox::environment::EnvHandleError),
    NoConfig,
}

impl Debug for EspxEnvError {
    fn fmt(&self, f: &mut Formatter<'_>) -> FmtResult {
        error_chain_fmt(self, f)
    }
}

impl Display for EspxEnvError {
    fn fmt(&self, f: &mut Formatter<'_>) -> FmtResult {
        write!(f, "{:?}", self)
    }
}
