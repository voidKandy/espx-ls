use crate::error::error_chain_fmt;
use std::fmt::{Debug, Display, Formatter, Result as FmtResult};

pub type CommandResult<T> = Result<T, CommandError>;

#[derive(thiserror::Error)]
pub enum CommandError {
    #[error(transparent)]
    Undefined(#[from] anyhow::Error),
    ParseFromComment(#[from] CommandParseError),
    RegistryFull,
    UnhandledLanguageExtension(String),
}

#[derive(thiserror::Error, Debug)]
pub enum CommandParseError {
    NoCommand(u8),
    NoScope(u8),
    AllWhitespace,
    NoScopeCharacter,
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
            Self::RegistryFull => "Registry Full".to_owned(),
            Self::UnhandledLanguageExtension(ext) => format!("Unhandled Languge Extension: {ext}"),
            Self::ParseFromComment(err) => format!("No command could be parsed from {err:?}"),
        };
        write!(f, "{}", display)
    }
}

impl Display for CommandParseError {
    fn fmt(&self, f: &mut Formatter<'_>) -> FmtResult {
        let str = match self {
            Self::NoCommand(id) => format!("No Command with id: {id}"),
            Self::NoScope(id) => format!("No Scope with id: {id}"),
            Self::AllWhitespace => "All Whitespace".to_owned(),
            Self::NoScopeCharacter => "No Scope Character".to_owned(),
        };
        write!(f, "{str}")
    }
}
