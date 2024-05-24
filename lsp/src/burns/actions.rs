use super::{error::BurnResult, EchoBurn};
use crate::{
    config::GLOBAL_CONFIG,
    espx_env::AgentID,
    handle::{operation_stream::BufferOpStreamSender, BufferOperation},
    parsing::get_prompt_on_line,
    state::GlobalState,
    store::walk_dir,
};
use anyhow::anyhow;
use espionox::{
    agents::{
        actions::{io_completion, stream_completion},
        memory::Message as EspxMessage,
    },
    language_models::openai::completions::streaming::{
        CompletionStreamStatus, StreamedCompletionHandler,
    },
};
use log::debug;
use lsp_types::{
    ApplyWorkspaceEditParams, HoverContents, MarkupKind, MessageType, Position, Range,
    ShowMessageParams, TextEdit, Url, WorkDoneProgress, WorkDoneProgressBegin, WorkDoneProgressEnd,
    WorkDoneProgressReport, WorkspaceEdit,
};
use serde::{Deserialize, Serialize};
use std::{collections::HashMap, fmt::format, path::PathBuf};
use tokio::sync::RwLockWriteGuard;

/// Action Burns are parsed from the document
#[derive(Clone, Serialize, Deserialize, Debug)]
pub struct ActionBurn {
    pub(super) typ: ActionType,
    pub(super) range: Range,
    pub(super) user_input: Option<String>,
    pub(super) replacement_text: String,
}

#[derive(Clone, Serialize, Deserialize, Debug, PartialEq, Eq)]
pub(super) enum ActionType {
    QuickPrompt,
    RagPrompt,
    WalkProject,
}

impl ActionType {
    fn all_variants() -> Vec<ActionType> {
        vec![
            ActionType::QuickPrompt,
            ActionType::RagPrompt,
            ActionType::WalkProject,
        ]
    }

    /// Gets trigger from GLOBAL_CONFIG, appends a whitespace
    fn trigger_string(&self) -> String {
        let actions_config = &GLOBAL_CONFIG.user_actions;
        match self {
            Self::QuickPrompt => format!("{} ", actions_config.quick_prompt.to_owned()),
            Self::RagPrompt => format!("{} ", actions_config.rag_prompt.to_owned()),
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
                            ActionType::QuickPrompt | ActionType::RagPrompt => {
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
            Self::QuickPrompt => {
                let work_done = WorkDoneProgressBegin {
                    title: "Quick Prompting Model".into(),
                    message: Some(String::from("Initializing")),
                    percentage: Some(0),
                    ..Default::default()
                };
                Some(BufferOperation::WorkDone(WorkDoneProgress::Begin(
                    work_done,
                )))
            }
            Self::RagPrompt => {
                let work_done = WorkDoneProgressBegin {
                    title: "RAG Prompting Model".into(),
                    message: Some(String::from("Initializing")),
                    percentage: Some(0),
                    ..Default::default()
                };
                Some(BufferOperation::WorkDone(WorkDoneProgress::Begin(
                    work_done,
                )))
            }
            Self::WalkProject => {
                let work_done = WorkDoneProgressBegin {
                    title: "Walking Project".into(),
                    message: Some(String::from("Initializing")),
                    percentage: Some(0),
                    ..Default::default()
                };
                Some(BufferOperation::WorkDone(WorkDoneProgress::Begin(
                    work_done,
                )))
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

        if self.typ == ActionType::RagPrompt {
            if let Some(db) = &state_guard.store.db {
                state_guard
                    .espx_env
                    .updater
                    .inner_write_lock()?
                    .refresh_update_with_similar_database_chunks(
                        &db.client,
                        &self
                            .user_input
                            .as_ref()
                            .expect("Why is there no user input?"),
                    )
                    .await?;
            }
        }

        match &self.typ {
            t @ ActionType::QuickPrompt | t @ ActionType::RagPrompt => {
                debug!("DOING IO PROMPT ACTION");

                state_guard
                    .espx_env
                    .updater
                    .inner_write_lock()?
                    .refresh_update_with_cache(&state_guard.store)
                    .await?;

                let agent = state_guard
                    .espx_env
                    .agents
                    .get_mut(&AgentID::Assistant)
                    .expect("why no agent");

                let trigger = if ActionType::RagPrompt == *t {
                    Some("updater")
                } else {
                    None
                };

                let mut response: StreamedCompletionHandler =
                    agent.do_action(stream_completion, (), trigger).await?;

                let work_done = WorkDoneProgressReport {
                    message: Some(String::from("Got Stream Completion Handler")),
                    ..Default::default()
                };
                sender
                    .send_operation(BufferOperation::WorkDone(WorkDoneProgress::Report(
                        work_done,
                    )))
                    .await?;

                while let Some(status) = response.receive(agent).await {
                    match status {
                        CompletionStreamStatus::Working(_) => {
                            let work_done = WorkDoneProgressReport {
                                message: Some(response.message_content.clone()),
                                ..Default::default()
                            };
                            sender
                                .send_operation(BufferOperation::WorkDone(
                                    WorkDoneProgress::Report(work_done),
                                ))
                                .await?;
                        }
                        CompletionStreamStatus::Finished => {
                            let work_done = WorkDoneProgressEnd {
                                message: Some(String::from("Finished")),
                                ..Default::default()
                            };
                            sender
                                .send_operation(BufferOperation::WorkDone(WorkDoneProgress::End(
                                    work_done,
                                )))
                                .await?;
                        }
                    }
                }

                let message = ShowMessageParams {
                    typ: MessageType::INFO,
                    message: response.message_content.clone(),
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
                        &response.message_content,
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
                let docs = walk_dir(PathBuf::from("."))?;
                debug!("GOT DOCS: {:?}", docs);
                let mut update_counter = 0;
                if let Some(ref mut db) = &mut state_guard.store.db {
                    for (i, (path, text)) in docs.iter().enumerate() {
                        let work_done = WorkDoneProgressReport {
                            message: Some(format!("Walking {}", path.display().to_string())),
                            percentage: Some((i as f32 / docs.len() as f32 * 100.0) as u32),
                            ..Default::default()
                        };
                        sender
                            .send_operation(BufferOperation::WorkDone(WorkDoneProgress::Report(
                                work_done,
                            )))
                            .await?;

                        let url = Url::parse(&format!("file:///{}", path.display().to_string()))
                            .expect("Failed to build URL");
                        db.client.update_doc_store(&text, url).await?;
                        update_counter += 1;
                    }

                    // db.read_all_docs_to_cache().await?;
                    // state_guard.espx_env.updater.inner_write_lock()?.refresh_update_with_similar_database_chunks(db, prompt).await?;
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

                let work_done = WorkDoneProgressEnd {
                    message: Some(String::from("Finished")),
                    ..Default::default()
                };
                sender
                    .send_operation(BufferOperation::WorkDone(WorkDoneProgress::End(work_done)))
                    .await?;
                let hover_contents = HoverContents::Markup(lsp_types::MarkupContent {
                    kind: MarkupKind::Markdown,
                    value: format!(
                        "Finished walking project, added {:?} docs to database.",
                        update_counter
                    ),
                });

                Ok(Some(EchoBurn {
                    content,
                    range,
                    hover_contents,
                }))
            }
        }
    }
}
