use super::{
    multiline::MultiLineActivation,
    singleline::{SingleLineActivation, SingleLineVariant},
    MultiLineVariant,
};
use crate::{
    handle::{
        buffer_operations::{BufferOpChannelSender, BufferOperation},
        error::HandleResult,
    },
    state::GlobalState,
    util::{self, OneOf},
};
use espionox::agents::Agent;
use lsp_server::RequestId;
use lsp_types::{HoverContents, Position, Range, Uri};
use serde::{Deserialize, Serialize};
use tokio::sync::RwLockWriteGuard;

#[derive(Clone, Serialize, Deserialize, Debug, PartialEq, Eq)]
pub enum Activation {
    Single(SingleLineActivation),
    Multi(MultiLineActivation),
}

#[derive(Clone, Serialize, Deserialize, Debug, PartialEq, Eq)]
pub struct BurnRange(lsp_types::Range);

impl From<lsp_types::Range> for BurnRange {
    fn from(value: lsp_types::Range) -> Self {
        Self(value)
    }
}

impl Into<lsp_types::Range> for BurnRange {
    fn into(self) -> lsp_types::Range {
        self.0
    }
}

pub(super) trait BurnActivationVariant: Sized + TryFrom<String> {
    fn all() -> Vec<Self>;
}

pub trait BurnActivation<V>
where
    V: BurnActivationVariant,
{
    fn doing_action_notification(&self) -> Option<BufferOperation>;
    fn trigger_pattern(&self) -> String;
    async fn activate(
        &mut self,
        uri: Uri,
        request_id: Option<RequestId>,
        position: Option<Position>,
        sender: &mut BufferOpChannelSender,
        agent: &mut Agent,
        state_guard: &mut RwLockWriteGuard<'_, GlobalState>,
    ) -> HandleResult<Option<HoverContents>>;
}

impl BurnRange {
    pub fn as_ref(&self) -> &lsp_types::Range {
        &self.0
    }

    pub fn as_mut(&mut self) -> &mut lsp_types::Range {
        &mut self.0
    }

    /// Checks for range overlap
    pub fn overlaps(&self, other: impl Into<Self>) -> bool {
        let other = Into::<Self>::into(other);
        let other_range: &lsp_types::Range = other.as_ref();
        let my_range: &lsp_types::Range = self.as_ref();

        my_range.start.line <= other_range.end.line && my_range.end.line >= other_range.start.line

        // && my_range.start.character <= other_range.end.character
        // && my_range.end.character >= other_range.start.character
    }

    pub fn position_is_in(&self, pos: Position) -> bool {
        self.0.start.line <= pos.line && self.0.end.line >= pos.line
    }
    /// Gives the distance of the start and end positions
    /// the distance of one line is considered a single character
    pub fn distance(&self, other: impl Into<Self>) -> (u32, u32) {
        let other = Into::<Self>::into(other);
        let mut start_dist = 0;
        start_dist += util::abs_difference(self.0.start.line, other.0.start.line);
        start_dist += util::abs_difference(self.0.start.character, other.0.start.character);
        let mut end_dist = 0;
        end_dist += util::abs_difference(self.0.end.line, other.0.end.line);
        end_dist += util::abs_difference(self.0.end.character, other.0.end.character);
        (start_dist, end_dist)
    }
}

impl Activation {
    pub fn range(&self) -> OneOf<&BurnRange, (&BurnRange, &BurnRange)> {
        match self {
            Self::Multi(a) => OneOf::Right((&a.start_range, &a.end_range)),
            Self::Single(a) => OneOf::Left(&a.range),
        }
    }
    pub fn trigger_diagnostic(&self) -> Option<String> {
        match self {
            Self::Multi(_) => None,
            Self::Single(a) => match a.variant {
                SingleLineVariant::RagPrompt => None,
                SingleLineVariant::QuickPrompt => None,
                SingleLineVariant::WalkProject => {
                    Some("Goto Def to trigger a directory walk".to_owned())
                }
                SingleLineVariant::LockDocIntoContext => {
                    Some("Document locked into agent context".to_owned())
                }
            },
        }
    }

    pub fn user_input_diagnostic(&self) -> Option<String> {
        match self {
            Self::Multi(_) => None,
            Self::Single(a) => match a.variant {
                SingleLineVariant::RagPrompt => Some("Goto Def to RAGPrompt agent".to_owned()),
                SingleLineVariant::QuickPrompt => Some("Goto Def to QuickPrompt agent".to_owned()),
                SingleLineVariant::WalkProject => None,
                SingleLineVariant::LockDocIntoContext => None,
            },
        }
    }

    #[tracing::instrument(name = "checking for activation overlap")]
    pub fn overlaps(&self, range: &Range) -> bool {
        match &self {
            Activation::Single(a) => a.range.overlaps(*range),
            Activation::Multi(a) => a.start_range.overlaps(*range) || a.end_range.overlaps(*range),
        }
    }

    pub fn is_in_position(&self, pos: &Position) -> bool {
        match &self {
            Activation::Single(a) => a.range.position_is_in(*pos),
            Activation::Multi(a) => {
                a.start_range.position_is_in(*pos) || a.end_range.position_is_in(*pos)
            }
        }
    }

    pub fn is_on_line(&self, line: u32) -> bool {
        match &self {
            Activation::Single(a) => a.range.as_ref().start.line == line,
            Activation::Multi(a) => {
                a.start_range.as_ref().start.line == line || a.end_range.as_ref().start.line == line
            }
        }
    }

    pub fn matches_variant(&self, other: &Self) -> bool {
        match (&self, &other) {
            (Activation::Multi(m), Activation::Multi(mo)) => match (&m.variant, &mo.variant) {
                (
                    MultiLineVariant::LockChunkIntoContext,
                    MultiLineVariant::LockChunkIntoContext,
                ) => true,
                _ => false,
            },
            (Activation::Single(s), Activation::Single(so)) => match (&s.variant, &so.variant) {
                (SingleLineVariant::QuickPrompt, SingleLineVariant::QuickPrompt)
                | (SingleLineVariant::RagPrompt, SingleLineVariant::RagPrompt)
                | (SingleLineVariant::WalkProject, SingleLineVariant::WalkProject) => true,
                _ => false,
            },
            _ => false,
        }
    }
}

mod tests {
    use lsp_types::{Position, Range};

    use super::BurnRange;

    #[test]
    fn burn_range_is_in_works() {
        let range = BurnRange::from(Range {
            start: Position {
                line: 5,
                character: 3,
            },
            end: Position {
                line: 6,
                character: 0,
            },
        });
        let other = BurnRange::from(Range {
            start: Position {
                line: 5,
                character: 10,
            },
            end: Position {
                line: 5,
                character: 20,
            },
        });

        assert!(other.overlaps(range.clone()));
        assert!(range.overlaps(other));

        let range = BurnRange::from(Range {
            start: Position {
                line: 4,
                character: 5,
            },
            end: Position {
                line: 4,
                character: 7,
            },
        });
        let other = BurnRange::from(Range {
            start: Position {
                line: 4,
                character: 0,
            },
            end: Position {
                line: 5,
                character: 0,
            },
        });

        assert!(other.overlaps(range.clone()));
        assert!(range.overlaps(other));
    }
}
