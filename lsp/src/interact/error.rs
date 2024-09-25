use crate::error::error_chain_fmt;
use std::fmt::{Debug, Display, Formatter, Result as FmtResult};

pub type InteractResult<T> = Result<T, InteractError>;

#[derive(thiserror::Error)]
pub enum InteractError {
    #[error(transparent)]
    Undefined(#[from] anyhow::Error),
    ParseFromComment(#[from] InteractParseError),
    RegistryFull,
    UnhandledLanguageExtension(String),
}

#[derive(thiserror::Error, Debug)]
pub enum InteractParseError {
    NoInteract(u8),
    NoScope(u8),
    AllWhitespace,
    NoScopeCharacter,
}

impl Debug for InteractError {
    fn fmt(&self, f: &mut Formatter<'_>) -> FmtResult {
        error_chain_fmt(self, f)
    }
}

impl Display for InteractError {
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

impl Display for InteractParseError {
    fn fmt(&self, f: &mut Formatter<'_>) -> FmtResult {
        let str = match self {
            Self::NoInteract(id) => format!("No Interact with id: {id}"),
            Self::NoScope(id) => format!("No Scope with id: {id}"),
            Self::AllWhitespace => "All Whitespace".to_owned(),
            Self::NoScopeCharacter => "No Scope Character".to_owned(),
        };
        write!(f, "{str}")
    }
}
