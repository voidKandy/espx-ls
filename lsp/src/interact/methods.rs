use lsp_types::Position;

use super::{lexer::Token, InteractError, InteractResult};

pub(super) const COMMAND_MASK: u8 = 0b0000_1111;
pub const COMMAND_PROMPT: u8 = 0b0;
pub const COMMAND_PUSH: u8 = 0b1;

pub(super) const SCOPE_MASK: u8 = 0b1111_0000;
pub const SCOPE_GLOBAL: u8 = 0b0;
pub const SCOPE_DOCUMENT: u8 = 0b0001_0000;

pub struct Interact;

fn u8_to_binary_string(num: u8) -> String {
    let mut result = String::with_capacity(8);
    for i in (0..8).rev() {
        if num & (1 << i) != 0 {
            result.push('1');
        } else {
            result.push('0');
        }
    }
    result
}

impl Interact {
    pub fn human_readable(id: u8) -> String {
        let command_str = match id & COMMAND_MASK {
            COMMAND_PUSH => "PUSH",
            COMMAND_PROMPT => "PROMPT",
            other => &format!("UNKNOWN COMMAND ID: {}", u8_to_binary_string(other)),
        };

        let scope_str = match id & SCOPE_MASK {
            SCOPE_GLOBAL => "GLOBAL",
            SCOPE_DOCUMENT => "DOCUMENT",
            other => &format!("UNKNOWN SCOPE ID: {}", u8_to_binary_string(other)),
        };

        format!("{command_str}_{scope_str}")
    }
    //
    // /// Splits single id into (COMMAND,SCOPE) to make pattern matching a little easier
    // /// only allows valid values to be returned
    // pub fn interract_tuple(id: u8) -> InteractResult<(u8, u8)> {
    //     let command = id & COMMAND_MASK;
    //     let scope = id & SCOPE_MASK;
    //
    //     if (command != COMMAND_PROMPT && command != COMMAND_PUSH)
    //         || (scope != SCOPE_GLOBAL && scope != SCOPE_DOCUMENT)
    //     {
    //         return Err(InteractError::InvalidInteractId(id));
    //     }
    //
    //     return Ok((command, scope));
    // }
}
