mod comment_str_map;
mod error;
pub mod parsing;
pub use error::{CommandError, CommandResult};

pub enum Command {
    Push,
    Prompt,
}

impl Command {
    pub const PUSH_CHAR: char = '@';
    pub const PROMPT_CHAR: char = '>';
}
