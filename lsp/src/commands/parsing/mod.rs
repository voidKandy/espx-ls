pub mod lexer;
use std::sync::LazyLock;

use lexer::ParsedComment;

use super::{CommandError, CommandResult};

/// Given a chunk of text and the language extenstion, returns a vec of all comments and their
/// posisitons
pub fn parse_chunk_for_comments(chunk: &str, ext: &str) -> CommandResult<Vec<ParsedComment>> {
    Ok(vec![])
}
