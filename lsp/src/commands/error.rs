use crate::error::error_chain_fmt;
use std::fmt::{Debug, Display, Formatter, Result as FmtResult};

pub type CommandResult<T> = Result<T, CommandError>;

#[derive(thiserror::Error)]
pub enum CommandError {
    #[error(transparent)]
    Undefined(#[from] anyhow::Error),
    UnhandledLanguageExtension(String),
}

impl Debug for CommandError {
    fn fmt(&self, f: &mut Formatter<'_>) -> FmtResult {
        error_chain_fmt(self, f)
    }
}

impl Display for CommandError {
    fn fmt(&self, f: &mut Formatter<'_>) -> FmtResult {
        let display = match self {
            Self::Undefined(err) => err.to_string(),
            Self::UnhandledLanguageExtension(ext) => format!("Unhandled Languge Extension: {ext}"),
        };
        write!(f, "{}", display)
    }
}
