use std::collections::HashMap;

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
use espionox::{
    agents::memory::{OtherRoleTo, ToMessage},
    prelude::*,
};
use lsp_server::RequestId;
use lsp_types::{
    ApplyWorkspaceEditParams, MessageType, Position, Range, ShowMessageParams, TextEdit, Uri,
    WorkspaceEdit,
};
use serde::{Deserialize, Serialize};
use tracing::{debug, warn};

#[derive(Clone, Serialize, Deserialize, Debug)]
pub enum MultiLineBurn {
    LockChunkIntoContext,
    LockDocIntoContext,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct TextOnLineRange {
    pub range: Range,
    pub text: String,
}

impl Burn for MultiLineBurn {
    fn all_variants() -> Vec<Self> {
        vec![Self::LockChunkIntoContext, Self::LockDocIntoContext]
    }

    fn trigger_string(&self) -> String {
        match self {
            Self::LockChunkIntoContext => GLOBAL_CONFIG
                .user_actions
                .lock_chunk_into_context
                .to_owned(),
            Self::LockDocIntoContext => GLOBAL_CONFIG.user_actions.lock_doc_into_context.to_owned(),
        }
        .to_string()
    }
    fn user_input_diagnostic(&self) -> Option<String> {
        None
    }

    fn trigger_diagnostic(&self) -> Option<String> {
        if let Self::LockDocIntoContext = self {
            return Some(String::from("document locked into agent context"));
        }
        None
    }

    fn doing_action_notification(&self) -> Option<BufferOperation> {
        None
    }

    async fn activate(
        &mut self,
        uri: Uri,
        _request_id: Option<RequestId>,
        _position: Option<Position>,
        sender: &mut BufferOpChannelSender,
        agent: &mut Agent,
        state_guard: &mut tokio::sync::RwLockWriteGuard<'_, crate::state::GlobalState>,
        // agent: &mut Agent,
    ) -> HandleResult<()> {
        let doc = state_guard.store.get_doc(&uri)?;
        let (inputs, _) = self.parse_for_user_inputs_and_triggers(&doc)?;

        match self {
            Self::LockChunkIntoContext => {
                let role = MessageRole::Other {
                    alias: "LockChunkIntoContext".to_owned(),
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
            Self::LockDocIntoContext => {
                let doc = state_guard.store.get_doc(&uri)?;
                let role = MessageRole::Other {
                    alias: "LockDockIntoContext".to_owned(),
                    coerce_to: OtherRoleTo::User,
                };
                agent.cache.mut_filter_by(&role, false);
                let message = doc.to_message(role);
                agent.cache.push(message);

                let all_lines_with_conflicting =
                    parsing::all_lines_with_pattern_with_char_positions(
                        &Self::LockChunkIntoContext.trigger_string(),
                        &doc,
                    );

                if !all_lines_with_conflicting.is_empty() {
                    warn!(
                        "doc lock multiline found conflicting burns: {:?}",
                        all_lines_with_conflicting
                    );
                    sender
                        .send_operation(BufferOperation::ShowMessage(ShowMessageParams {
                            typ: MessageType::WARNING,
                            message:
                                "Chunk locks cannot be included on a document that has been locked"
                                    .to_owned(),
                        }))
                        .await?;
                }

                let mut text_edits = vec![];
                all_lines_with_conflicting
                    .into_iter()
                    .for_each(|(line, char)| {
                        text_edits.push(TextEdit {
                            range: Range {
                                start: Position { line, character: 0 },
                                end: Position {
                                    line,
                                    character: char
                                        + Self::LockChunkIntoContext.trigger_string().len() as u32,
                                },
                            },
                            new_text: String::new(),
                        })
                    });

                let mut changes = HashMap::new();
                changes.insert(uri.clone(), text_edits);
                let edit_params = ApplyWorkspaceEditParams {
                    label: None,
                    edit: WorkspaceEdit {
                        changes: Some(changes),
                        ..Default::default()
                    },
                };

                sender.send_operation(edit_params.into()).await?;
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

        let mut triggers = vec![];

        match &self {
            Self::LockDocIntoContext => {
                if idxs.len() > 1 {
                    warn!("should not have mutliple doc locks on single document");
                }
                for (line_idx, char_idx) in idxs {
                    triggers.push(TextOnLineRange {
                        range: Range {
                            start: Position {
                                line: line_idx,
                                character: char_idx,
                            },
                            end: Position {
                                line: line_idx,
                                character: char_idx + self.trigger_string().len() as u32,
                            },
                        },
                        text: self.trigger_string(),
                    });
                }
                Ok((vec![], triggers))
            }

            Self::LockChunkIntoContext => {
                let mut inputs = vec![];
                if idxs.len() % 2 != 0 {
                    warn!("there must be an unclosed pattern for a multiline burn as there is an uneven amount of matching lines");
                }

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
                    user_input_buffer.push_str(
                        lines
                            .iter()
                            .nth(first_line_idx as usize + 1)
                            .expect("This should be ok"),
                    );

                    for k in 1..(last_line_idx - first_line_idx) - 1 {
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
                Ok((inputs, triggers))
            }
        }
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

        let (user_inputs, triggers) = MultiLineBurn::LockChunkIntoContext
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
