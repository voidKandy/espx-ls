use super::Burn;
use crate::{
    config::GLOBAL_CONFIG,
    handle::{
        buffer_operations::{BufferOpChannelSender, BufferOperation},
        error::HandleResult,
    },
    parsing,
};
use anyhow::anyhow;
use espionox::{agents::memory::OtherRoleTo, prelude::*};
use lsp_server::RequestId;
use lsp_types::{Position, Range, Uri};
use serde::{Deserialize, Serialize};
use tracing::{debug, warn};

#[derive(Clone, Serialize, Deserialize, Debug)]
pub enum MultiLineBurn {
    LockIntoContext,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct TextOnLineRange {
    pub range: Range,
    pub text: String,
}

impl Burn for MultiLineBurn {
    fn all_variants() -> Vec<Self> {
        vec![Self::LockIntoContext]
    }

    fn trigger_string(&self) -> String {
        match self {
            Self::LockIntoContext => GLOBAL_CONFIG.user_actions.lock_into_context.to_owned(),
        }
        .to_string()
    }

    fn user_input_diagnostic(&self) -> String {
        match self {
            Self::LockIntoContext => "Content locked into assistant agent context",
        }
        .to_string()
    }

    fn trigger_diagnostic(&self) -> String {
        match self {
            Self::LockIntoContext => "",
        }
        .to_string()
    }

    fn doing_action_notification(&self) -> Option<BufferOperation> {
        match self {
            Self::LockIntoContext => None,
        }
    }

    async fn activate_on_document(
        &mut self,
        uri: Uri,
        _request_id: Option<RequestId>,
        _position: Option<Position>,
        _sender: &mut BufferOpChannelSender,
        agent: &mut Agent,
        state_guard: &mut tokio::sync::RwLockWriteGuard<'_, crate::state::GlobalState>,
        // agent: &mut Agent,
    ) -> HandleResult<()> {
        let doc = state_guard.store.get_doc(&uri)?;
        let (inputs, _) = self.parse_for_user_inputs_and_triggers(&doc)?;

        match self {
            Self::LockIntoContext => {
                let role = MessageRole::Other {
                    alias: "LockIntoContext".to_owned(),
                    coerce_to: OtherRoleTo::User,
                };
                agent.cache.mut_filter_by(&role, false);
                agent.cache.push(Message {
                    role,
                    content: inputs
                        .iter()
                        .map(|i| i.text.as_str())
                        .collect::<Vec<&str>>()
                        .join(""),
                });
            }
        }

        Ok(())
    }
}

impl MultiLineBurn {
    pub fn parse_for_user_inputs_and_triggers(
        &self,
        text: &str,
    ) -> anyhow::Result<(Vec<TextOnLineRange>, Vec<TextOnLineRange>)> {
        let mut idxs =
            parsing::all_lines_with_pattern_with_char_positions(&self.trigger_string(), text);
        if idxs.is_empty() {
            return Err(anyhow!(
                "No lines matching pattern: {}",
                self.trigger_string()
            ));
        }
        if idxs.len() % 2 != 0 {
            warn!("there must be an unclosed pattern for a multiline burn as there is an uneven amount of matching lines");
        }

        let mut inputs = vec![];
        let mut triggers = vec![];

        for _ in 0..idxs.len() / 2 {
            let (first_line_idx, first_char_idx) = idxs.remove(0);
            let (last_line_idx, last_char_idx) = idxs.remove(0);

            triggers.push(TextOnLineRange {
                range: Range {
                    start: Position {
                        line: first_line_idx,
                        character: first_char_idx,
                    },
                    end: Position {
                        line: first_line_idx,
                        character: first_char_idx + self.trigger_string().len() as u32,
                    },
                },
                text: self.trigger_string(),
            });

            let mut user_input_buffer = String::new();
            let lines: Vec<&str> = text.lines().collect();
            debug!("Lines: {:?}", lines);
            debug!(
                "pushing: {:?}\nfirst line idx: {}",
                lines
                    .iter()
                    .nth(first_line_idx as usize + 1)
                    .expect("This should be ok"),
                first_line_idx + 1
            );
            user_input_buffer.push_str(
                lines
                    .iter()
                    .nth(first_line_idx as usize + 1)
                    .expect("This should be ok"), // .split_at(first_char_idx as usize + self.trigger_string().len())
                                                  // .1,
            );

            for k in 1..(last_line_idx - first_line_idx) - 1 {
                debug!(
                    "pushing: {:?}",
                    lines
                        .iter()
                        .nth(first_line_idx as usize + k as usize)
                        .expect("This should be ok"),
                );
                user_input_buffer.push_str(
                    lines
                        .iter()
                        .nth(first_line_idx as usize + k as usize)
                        .expect("this should be ok"),
                )
            }

            if !user_input_buffer.is_empty() {
                inputs.push(TextOnLineRange {
                    range: Range {
                        start: Position {
                            line: first_line_idx + 1,
                            character: 0,
                        },
                        end: Position {
                            line: first_line_idx + user_input_buffer.lines().count() as u32,
                            character: user_input_buffer.len() as u32,
                        },
                    },
                    text: user_input_buffer,
                })
            }

            triggers.push(TextOnLineRange {
                range: Range {
                    start: Position {
                        line: last_line_idx,
                        character: last_char_idx,
                    },
                    end: Position {
                        line: last_line_idx,
                        character: last_char_idx + self.trigger_string().len() as u32,
                    },
                },
                text: self.trigger_string(),
            });
        }

        return Ok((inputs, triggers));
    }
}

mod tests {
    use lsp_types::Position;
    use tracing::info;

    use crate::{error::init_test_tracing, state::burns::multiline::TextOnLineRange};

    use super::MultiLineBurn;

    #[test]
    fn correctly_parses_text_for_lines() {
        init_test_tracing();
        let input = r#"
// --$$$--
this is text that should be considered input
// --$$$--
this text should not be considered input
        "#;

        let (user_inputs, triggers) = MultiLineBurn::LockIntoContext
            .parse_for_user_inputs_and_triggers(input)
            .unwrap();

        let expected_triggers = vec![
            TextOnLineRange {
                text: "--$$$--".to_owned(),
                range: lsp_types::Range {
                    start: Position {
                        line: 1,
                        character: 3,
                    },
                    end: Position {
                        line: 1,
                        character: 10,
                    },
                },
            },
            TextOnLineRange {
                text: "--$$$--".to_owned(),
                range: lsp_types::Range {
                    start: Position {
                        line: 3,
                        character: 3,
                    },
                    end: Position {
                        line: 3,
                        character: 10,
                    },
                },
            },
        ];

        let expected_input = TextOnLineRange {
            text: "this is text that should be considered input".to_owned(),
            range: lsp_types::Range {
                start: Position {
                    line: 2,
                    character: 0,
                },
                end: Position {
                    line: 2,
                    character: 44,
                },
            },
        };

        assert_eq!(expected_triggers, triggers);
        assert_eq!(expected_input, user_inputs[0]);

        // info!("test output: {:?}", output);
    }
}
