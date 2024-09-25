use std::sync::LazyLock;

use crate::{
    handle::buffer_operations::{BufferOpChannelSender, BufferOpChannelStatus},
    state::LspState,
};

pub const COMMAND_MASK: u8 = 0b0000_1111;
pub const COMMAND_PROMPT: u8 = 0b0;
pub const COMMAND_PUSH: u8 = 0b1;

pub const SCOPE_MASK: u8 = 0b1111_0000;
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
    pub fn hover_str(id: u8) -> String {
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

    pub fn goto_def_fn(id: u8, sender: &mut BufferOpChannelSender) {}
}

