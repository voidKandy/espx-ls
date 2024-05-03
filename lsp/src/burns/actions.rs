use super::{error::BurnResult, EchoBurn};
use crate::{
    config::GLOBAL_CONFIG,
    espx_env::agents::{get_inner_agent_handle, inner::InnerAgent},
    handle::{operation_stream::BufferOpStreamSender, BufferOperation},
    parsing::get_prompt_on_line,
    state::GlobalState,
    store::database::docs::DBDocument,
};
use espionox::{
    agents::memory::Message as EspxMessage,
    environment::{dispatch::EnvNotification, DispatchError, EnvError, EnvHandleError},
};
use log::debug;
use lsp_types::{
    ApplyWorkspaceEditParams, HoverContents, MarkupKind, MessageType, Position, Range,
    ShowMessageParams, TextEdit, Url, WorkspaceEdit,
};
use serde::{Deserialize, Serialize};
use std::{collections::HashMap, path::PathBuf, time::Duration};
use tokio::{sync::RwLockWriteGuard, time::sleep};

/// Action Burns are parsed from the document
#[derive(Clone, Serialize, Deserialize, Debug)]
pub struct ActionBurn {
    pub(super) typ: ActionType,
    pub(super) range: Range,
    pub(super) user_input: Option<String>,
    pub(super) replacement_text: String,
}

#[derive(Clone, Serialize, Deserialize, Debug)]
pub(super) enum ActionType {
    IoPrompt,
    WalkProject,
}

impl ActionType {
    fn all_variants() -> Vec<ActionType> {
        vec![ActionType::IoPrompt, ActionType::WalkProject]
    }

    /// Gets trigger from GLOBAL_CONFIG, appends a whitespace
    fn trigger_string(&self) -> String {
        let actions_config = &GLOBAL_CONFIG.user_actions;
        match self {
            Self::IoPrompt => format!("{} ", actions_config.io_trigger.to_owned()),
            Self::WalkProject => actions_config.walk_project.to_owned(),
        }
    }

    /// Parses document for all actions
    pub(super) fn parse_for_actions(text: &str) -> Vec<ActionBurn> {
        let mut action_vec = vec![];
        for typ in Self::all_variants().into_iter() {
            let trigger_string = typ.trigger_string();
            for (i, l) in text.lines().into_iter().enumerate() {
                if l.contains(&trigger_string) {
                    if let Some((replacement_text, prompt)) = get_prompt_on_line(l, &trigger_string)
                    {
                        match typ {
                            ActionType::IoPrompt => {
                                log::info!("PARSED PROMPT: {}", prompt);
                                let line = i as u32;
                                let start_character_position =
                                    (replacement_text.len() + trigger_string.len()) as u32;
                                let start = Position {
                                    line,
                                    character: start_character_position,
                                };
                                let end = Position {
                                    line,
                                    character: start_character_position + prompt.len() as u32,
                                };
                                action_vec.push(ActionBurn {
                                    typ: typ.clone(),
                                    replacement_text,
                                    user_input: Some(prompt),
                                    range: Range { start, end },
                                })
                            }
                            ActionType::WalkProject => {
                                let line = i as u32;
                                let start_character_position = replacement_text.len() as u32;

                                let start = Position {
                                    line,
                                    character: start_character_position,
                                };
                                let end = Position {
                                    line,
                                    character: start_character_position
                                        + trigger_string.len() as u32
                                        + prompt.len() as u32,
                                };
                                action_vec.push(ActionBurn {
                                    typ: typ.clone(),
                                    replacement_text,
                                    user_input: None,
                                    range: Range { start, end },
                                })
                            }
                        }
                    }
                }
            }
        }
        action_vec
    }

    /// Notification sent to client when action is being done
    fn doing_action_notification(&self) -> Option<BufferOperation> {
        match self {
            Self::IoPrompt => {
                let show_message = ShowMessageParams {
                    typ: MessageType::LOG,
                    message: String::from("Prompting Agent"),
                };
                Some(BufferOperation::ShowMessage(show_message))
            }
            Self::WalkProject => {
                let show_message = ShowMessageParams {
                    typ: MessageType::LOG,
                    message: String::from("Walking Project"),
                };
                Some(BufferOperation::ShowMessage(show_message))
            }
        }
    }
}

impl ActionBurn {
    /// When action is done, the text is removed from the buffer
    pub(super) fn workspace_edit(&self, url: Url) -> WorkspaceEdit {
        let mut changes = HashMap::new();
        let range = Range {
            start: Position {
                line: self.range.start.line,
                character: 0,
            },
            end: Position {
                line: self.range.end.line,
                character: self.range.end.character + 1,
            },
        };

        let textedit = TextEdit {
            range,
            new_text: format!("{}", self.replacement_text),
        };

        changes.insert(url, vec![textedit]);

        WorkspaceEdit {
            changes: Some(changes),
            ..Default::default()
        }
    }

    /// Does inner action and converts to echo
    pub(super) async fn do_action(
        &mut self,
        sender: &mut BufferOpStreamSender,
        url: Url,
        state_guard: &mut RwLockWriteGuard<'_, GlobalState>,
    ) -> BurnResult<Option<EchoBurn>> {
        if let Some(op) = self.typ.doing_action_notification() {
            sender.send_operation(op).await?;
        }

        let edit_params = ApplyWorkspaceEditParams {
            label: None,
            edit: self.workspace_edit(url.clone()),
        };
        sender.send_operation(edit_params.into()).await?;

        match self.typ {
            ActionType::IoPrompt => {
                debug!("DOING IO PROMPT ACTION");
                let handle = get_inner_agent_handle(InnerAgent::Assistant).unwrap();

                if !state_guard.espx_env.env_handle.is_running() {
                    let _ = state_guard.espx_env.env_handle.spawn();
                }

                let ticket = handle
                    .request_io_completion(EspxMessage::new_user(
                        &self.user_input.as_ref().unwrap(),
                    ))
                    .await
                    .map_err(|err| {
                        EnvHandleError::from(EnvError::from(DispatchError::from(err)))
                    })?;

                let response: EnvNotification = state_guard
                    .espx_env
                    .env_handle
                    .wait_for_notification(&ticket)
                    .await?;
                let response: &EspxMessage = response.extract_body().try_into()?;

                let message = ShowMessageParams {
                    typ: MessageType::INFO,
                    message: response.content.clone(),
                };

                sender.send_operation(message.into()).await?;

                let content = EchoBurn::generate_placeholder();
                let range = Range {
                    start: Position {
                        line: self.range.start.line,
                        character: self.replacement_text.len() as u32,
                    },
                    end: Position {
                        line: self.range.end.line,

                        character: self.replacement_text.len() as u32 + 1,
                    },
                };

                let hover_contents = HoverContents::Markup(lsp_types::MarkupContent {
                    kind: MarkupKind::Markdown,
                    value: [
                        "# User prompt: ",
                        &self.user_input.as_ref().unwrap(),
                        "# Assistant Response: ",
                        &response.content,
                    ]
                    .join("\n"),
                });

                Ok(Some(EchoBurn {
                    content,
                    range,
                    hover_contents,
                }))
            }
            ActionType::WalkProject => {
                debug!("DOING WALK PROJECT ACTION");
                let docs = state_guard.store.updater.walk_dir(PathBuf::from("."))?;
                debug!("GOT DOCS: {:?}", docs);
                let mut update_counter = 0;
                if let Some(db) = &state_guard.store.db {
                    for (path, text) in docs {
                        let url = Url::parse(&format!("file:///{}", path.display().to_string()))
                            .expect("Failed to build URL");
                        db.update_doc_store(&text, url).await?;
                        update_counter += 1;
                    }
                }

                let content = EchoBurn::generate_placeholder();
                let range = Range {
                    start: Position {
                        line: self.range.start.line,
                        character: self.replacement_text.len() as u32,
                    },
                    end: Position {
                        line: self.range.end.line,
                        character: self.replacement_text.len() as u32 + 1,
                    },
                };

                let hover_contents = HoverContents::Markup(lsp_types::MarkupContent {
                    kind: MarkupKind::Markdown,
                    value: format!(
                        "Finished walking project, added {:?} docs to database.",
                        update_counter
                    ),
                });

                // sender.send_operation(message.into()).await?;

                Ok(Some(EchoBurn {
                    content,
                    range,
                    hover_contents,
                }))
            }
        }
    }
}
