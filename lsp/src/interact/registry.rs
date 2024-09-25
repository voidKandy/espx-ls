use std::collections::HashMap;

use super::{
    error::{InteractError, InteractResult},
    methods::*,
};

#[derive(Debug, Clone, Hash, PartialEq, Eq)]
pub enum InteractCharacter {
    Scope(char),
    Command(char),
}

impl AsRef<char> for InteractCharacter {
    fn as_ref(&self) -> &char {
        match self {
            Self::Command(c) => &c,
            Self::Scope(c) => &c,
        }
    }
}

#[derive(Debug)]
pub struct InteractRegistry(HashMap<InteractCharacter, u8>);

type Ch = InteractCharacter;
impl Default for InteractRegistry {
    fn default() -> Self {
        let mut registered = HashMap::new();
        registered.insert(Ch::Scope('_'), SCOPE_GLOBAL);
        registered.insert(Ch::Scope('^'), SCOPE_DOCUMENT);

        registered.insert(Ch::Command('+'), COMMAND_PUSH);
        registered.insert(Ch::Command('@'), COMMAND_PROMPT);
        Self(registered)
    }
}

impl InteractRegistry {
    pub fn register_scope(&mut self, char: &char) -> InteractResult<()> {
        let max = self.max_scope_id();

        let id =
            Self::increment_masked_value(max, SCOPE_MASK).ok_or(InteractError::RegistryFull)?;

        // let scope = ScopeInfo {
        //     id,
        //     char: *char,
        //     // name
        // };

        self.0.insert(Ch::Scope(*char), id);
        Ok(())
    }

    fn increment_masked_value(value: u8, mask: u8) -> Option<u8> {
        if value == mask {
            return None;
        }
        let masked_value = value & mask;

        let shift_amount = mask.trailing_zeros();
        let incremented_value = ((masked_value >> shift_amount) + 1) << shift_amount;

        let new_value = (value & !mask) | (incremented_value & mask);

        if new_value > mask {
            panic!("overflow when incrementing masked value");
        }

        Some(new_value)
    }

    fn max_scope_id(&self) -> u8 {
        *self
            .0
            .values()
            .into_iter()
            .max()
            .expect("no shot this fails")
    }

    pub fn try_get_interact(&self, string: &String) -> Option<u8> {
        let first_non_whitespace_pos = string.chars().position(|c| !c.is_whitespace())?;

        let first_non_whitespace = string.chars().nth(first_non_whitespace_pos)?;
        let command_id: u8 = *self
            .0
            .keys()
            .find(|ch| Ch::Command(first_non_whitespace) == **ch)
            .and_then(|i| self.0.get(i))?;

        let next_char = string.chars().nth(first_non_whitespace_pos + 1)?;
        let scope_id: u8 = *self
            .0
            .keys()
            .find(|ch| Ch::Scope(next_char) == **ch)
            .and_then(|i| self.0.get(i))?;

        Some(command_id + scope_id)
    }
}
