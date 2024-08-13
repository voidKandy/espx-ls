use super::{
    activations::{BurnActivation, BurnActivationVariant, BurnRange},
    error::BurnError,
};
use crate::{
    config::GLOBAL_CONFIG,
    handle::{
        buffer_operations::BufferOperation,
        error::{HandleError, HandleResult},
    },
    parsing,
    state::{espx::stream_completion_with_rag, store::walk_dir, GlobalState},
};
use anyhow::anyhow;
use espionox::{
    agents::{
        actions::stream_completion,
        memory::{Message, OtherRoleTo, ToMessage},
        Agent,
    },
    language_models::completions::streaming::{CompletionStreamStatus, ProviderStreamHandler},
    prelude::{ListenerTrigger, MessageRole},
};
use lsp_server::RequestId;
use lsp_types::{
    ApplyWorkspaceEditParams, GotoDefinitionResponse, HoverContents, Location, MessageType,
    Position, Range, ShowMessageParams, TextEdit, Uri, WorkDoneProgress, WorkDoneProgressBegin,
    WorkspaceEdit,
};
use serde::{Deserialize, Serialize};
use std::{collections::HashMap, path::PathBuf, str::FromStr};
use tokio::sync::RwLockWriteGuard;
use tracing::{debug, warn};

#[derive(Clone, Serialize, Deserialize, Debug, PartialEq, Eq)]
pub enum SingleLineVariant {
    QuickPrompt,
    RagPrompt,
    WalkProject,
    LockDocIntoContext,
}

#[derive(Clone, Serialize, Deserialize, Debug, PartialEq, Eq)]
enum SingleLineState {
    Initial,
    Activated,
}

#[derive(Clone, Serialize, Deserialize, Debug, PartialEq, Eq)]
pub struct SingleLineActivation {
    pub variant: SingleLineVariant,
    pub range: BurnRange,
    state: SingleLineState,
}

impl BurnActivationVariant for SingleLineVariant {
    fn all() -> Vec<SingleLineVariant> {
        vec![
            SingleLineVariant::RagPrompt,
            SingleLineVariant::QuickPrompt,
            SingleLineVariant::WalkProject,
            SingleLineVariant::LockDocIntoContext,
        ]
    }
}

impl TryFrom<String> for SingleLineVariant {
    type Error = BurnError;
    fn try_from(value: String) -> Result<Self, Self::Error> {
        let str = value.as_str();
        let actions_config = &GLOBAL_CONFIG.user_actions;
        match str {
            _ if str == actions_config.quick_prompt || str == actions_config.quick_prompt_echo => {
                Ok(SingleLineVariant::QuickPrompt)
            }
            _ if str == actions_config.rag_prompt || str == actions_config.rag_prompt_echo => {
                Ok(SingleLineVariant::RagPrompt)
            }
            _ if str == actions_config.walk_project || str == actions_config.walk_project_echo => {
                Ok(SingleLineVariant::WalkProject)
            }
            _ if str == actions_config.lock_doc_into_context
                || str == actions_config.lock_doc_echo =>
            {
                Ok(SingleLineVariant::LockDocIntoContext)
            }

            _ => Err(anyhow!("cannot create variant").into()),
        }
    }
}

impl BurnActivation<SingleLineVariant> for SingleLineActivation {
    fn doing_action_notification(&self) -> Option<BufferOperation> {
        match self.variant {
            SingleLineVariant::QuickPrompt => {
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
            SingleLineVariant::RagPrompt => {
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
            SingleLineVariant::WalkProject => {
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
            SingleLineVariant::LockDocIntoContext => None,
        }
    }
    fn trigger_pattern(&self) -> String {
        let actions_config = &GLOBAL_CONFIG.user_actions;
        match self.state {
            SingleLineState::Activated => match self.variant {
                SingleLineVariant::RagPrompt => actions_config.rag_prompt_echo.to_owned(),
                SingleLineVariant::QuickPrompt => actions_config.quick_prompt_echo.to_owned(),
                SingleLineVariant::WalkProject => actions_config.walk_project_echo.to_owned(),
                SingleLineVariant::LockDocIntoContext => actions_config.lock_doc_echo.to_owned(),
            },
            SingleLineState::Initial => match self.variant {
                SingleLineVariant::QuickPrompt => actions_config.quick_prompt.to_owned(),
                SingleLineVariant::RagPrompt => actions_config.rag_prompt.to_owned(),
                SingleLineVariant::WalkProject => actions_config.walk_project.to_owned(),
                SingleLineVariant::LockDocIntoContext => {
                    actions_config.lock_doc_into_context.to_owned()
                }
            },
        }
    }

    async fn activate(
        &mut self,
        uri: Uri,
        request_id: Option<RequestId>,
        position: Option<Position>,
        sender: &mut crate::handle::buffer_operations::BufferOpChannelSender,
        agent: &mut Agent,
        state_guard: &mut RwLockWriteGuard<'_, GlobalState>,
    ) -> HandleResult<Option<HoverContents>> {
        debug!(
            "activating single line burn on document: {:?} at position: {:?}",
            uri.as_str(),
            position
        );

        if let SingleLineVariant::LockDocIntoContext = self.variant {
            let doc = state_guard.store.get_doc(&uri)?;

            let conflicting_burn_pattern = &GLOBAL_CONFIG.user_actions.lock_chunk_into_context;
            if !doc.contains(conflicting_burn_pattern) {
                warn!(
                    "doc lock found conflicting burn pattern: {:?}",
                    conflicting_burn_pattern
                );
                sender
                    .send_operation(BufferOperation::ShowMessage(ShowMessageParams {
                        typ: MessageType::WARNING,
                        message:
                            "Chunk locks cannot be included on a document that has been locked"
                                .to_owned(),
                    }))
                    .await?;

                let mut text_edits = vec![];
                parsing::all_lines_with_pattern_with_char_positions(
                    &doc,
                    &conflicting_burn_pattern,
                )
                .into_iter()
                .for_each(|(line, char)| {
                    text_edits.push(TextEdit {
                        range: Range {
                            start: Position {
                                line: line as u32,
                                character: 0,
                            },
                            end: Position {
                                line: line as u32,
                                character: (char + self.trigger_pattern().len()) as u32,
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

            let role = MessageRole::Other {
                alias: "LockDockIntoContext".to_owned(),
                coerce_to: OtherRoleTo::User,
            };
            agent.cache.mut_filter_by(&role, false);
            let message = doc.to_message(role);
            agent.cache.push(message);

            self.try_change_state(uri, sender).await?;

            return Ok(Some(HoverContents::Scalar(
                lsp_types::MarkedString::String(String::from(
                    "This document has been locked into agent context",
                )),
            )));
        }

        let request_id = request_id.ok_or(anyhow!(
            "request ID should be some when activating single line burns"
        ))?;
        let position = position.ok_or(anyhow!(
            "position should be some when activating single line burns"
        ))?;

        let doc = state_guard
            .store
            .get_doc(&uri)
            .map_err(|e| anyhow!("Could not get document: {:?}", e))?;
        debug!("got position, request ID and doc");

        if self.range.position_is_in(position) {
            debug!("activating for trigger",);
            return self
                .goto_definition_on_trigger(request_id, &position, uri, sender, agent, state_guard)
                .await
                .map(|opt| {
                    opt.and_then(|content| {
                        Some(HoverContents::Scalar(lsp_types::MarkedString::String(
                            content,
                        )))
                    })
                });
        } else {
            let doc_line = doc
                .lines()
                .nth(position.line as usize)
                .ok_or(anyhow!("could not get doc line: {}", position.line))?;

            if let Some(user_input) =
                parsing::slices_after_pattern(&doc_line, &self.trigger_pattern())
                    .and_then(|vec| Some(vec[0].to_owned()))
            {
                debug!("activating for input",);
                return self
                    .goto_definition_on_input(
                        request_id,
                        &position,
                        &user_input.text,
                        uri,
                        sender,
                        agent,
                        state_guard,
                    )
                    .await
                    .map(|opt| {
                        opt.and_then(|content| {
                            Some(HoverContents::Scalar(lsp_types::MarkedString::String(
                                content,
                            )))
                        })
                    });
            }
        }

        Ok(None)
    }
}

impl SingleLineActivation {
    pub fn new(variant: SingleLineVariant, pat: &str, range: impl Into<BurnRange>) -> Self {
        let actions_config = &GLOBAL_CONFIG.user_actions;
        let state = match pat {
            _ if pat == actions_config.lock_chunk_into_context
                || pat == actions_config.lock_doc_into_context
                || pat == actions_config.rag_prompt
                || pat == actions_config.walk_project
                || pat == actions_config.quick_prompt =>
            {
                SingleLineState::Initial
            }
            _ if pat == actions_config.lock_doc_echo
                || pat == actions_config.rag_prompt_echo
                || pat == actions_config.walk_project_echo
                || pat == actions_config.quick_prompt_echo =>
            {
                SingleLineState::Activated
            }
            p => panic!("why was {} passed into SingleLineActivation::new??", p),
        };
        Self {
            state,
            variant,
            range: range.into(),
        }
    }
    async fn try_change_state(
        &mut self,
        uri: Uri,
        sender: &mut crate::handle::buffer_operations::BufferOpChannelSender,
    ) -> HandleResult<()> {
        if self.state == SingleLineState::Initial {
            self.state = SingleLineState::Activated;

            let mut changes = HashMap::new();
            changes.insert(
                uri.clone(),
                vec![TextEdit {
                    range: *self.range.as_ref(),
                    new_text: self.trigger_pattern().to_owned(),
                }],
            );
            let edit_params = ApplyWorkspaceEditParams {
                label: None,
                edit: WorkspaceEdit {
                    changes: Some(changes),
                    ..Default::default()
                },
            };

            sender.send_operation(edit_params.into()).await?;

            self.range = Range {
                start: self.range.as_ref().start,
                end: Position {
                    line: self.range.as_ref().end.line,
                    character: self.range.as_ref().start.character
                        + self.trigger_pattern().len() as u32,
                },
            }
            .into();
        }
        Ok(())
    }
    #[allow(unused)]
    pub async fn goto_definition_on_trigger(
        &mut self,
        request_id: RequestId,
        position: &Position,
        uri: Uri,
        sender: &mut crate::handle::buffer_operations::BufferOpChannelSender,
        agent: &mut Agent,
        state_guard: &mut RwLockWriteGuard<'_, GlobalState>,
    ) -> HandleResult<Option<String>> {
        debug!("activating burn on trigger: {:?}", self);
        match &self.variant {
            v @ SingleLineVariant::QuickPrompt | v @ SingleLineVariant::RagPrompt => {
                if *v == SingleLineVariant::RagPrompt {}
                let path = &GLOBAL_CONFIG.conversation_file();
                let uri = Uri::from_str(path.to_str().expect("path is not valid unicode"))
                    .map_err(|err| anyhow!("error converting path to uri: {:?}", err))?;
                let op = BufferOperation::GotoFile {
                    id: request_id,
                    response: GotoDefinitionResponse::Scalar(Location {
                        uri,
                        range: Range::default(),
                    }),
                };
                sender.send_operation(op).await?;
            }
            SingleLineVariant::WalkProject => {
                let docs = walk_dir(PathBuf::from("."))
                    .map_err(|err| anyhow!("error walking dir: {:?}", err))?;
                debug!("GOT DOCS: {:?}", docs);
                sender.start_work_done(None);
                let mut update_counter = 0;
                for (i, (path, text)) in docs.iter().enumerate() {
                    sender
                        .send_work_done_report(
                            Some(&format!("Adding {} to memory...", path.display())),
                            Some((i as f32 / docs.len() as f32 * 100.0) as u32),
                        )
                        .await?;

                    let uri = Uri::from_str(&format!("file://{}", path.display().to_string()))
                        .expect("Failed to build uri");

                    // if let Some(db) = &state_guard.store.db {
                    //     state_guard.store.update_doc(&text, uri);
                    //     update_counter += 1;
                    // }
                }

                self.try_change_state(uri.clone(), sender).await?;
                sender.send_work_done_end(None).await?;
                return Ok(Some(format!(
                    "Finished walking project, added {:?} docs to database.",
                    update_counter
                )));
            }
            SingleLineVariant::LockDocIntoContext => {}
        }
        Ok(None)
    }

    #[allow(unused)]
    pub async fn goto_definition_on_input(
        &mut self,
        request_id: RequestId,
        position: &Position,
        input: &str,
        uri: Uri,
        sender: &mut crate::handle::buffer_operations::BufferOpChannelSender,
        agent: &mut Agent,
        state_guard: &mut RwLockWriteGuard<'_, GlobalState>,
    ) -> HandleResult<Option<String>> {
        debug!("activating burn on user input: {:?}", self);
        if let Some(op) = self.doing_action_notification() {
            sender.send_operation(op).await?;
        }

        match &self.variant {
            t @ SingleLineVariant::QuickPrompt | t @ SingleLineVariant::RagPrompt => {
                let edit_range = Range {
                    start: Position {
                        line: self.range.as_ref().start.line,
                        character: self.range.as_ref().start.character,
                    },
                    end: Position {
                        line: self.range.as_ref().end.line,
                        character: input.len() as u32
                            + self.range.as_ref().start.character
                            + self.trigger_pattern().len() as u32
                            + 1,
                    },
                };

                let mut changes = HashMap::new();
                changes.insert(
                    uri.clone(),
                    vec![TextEdit {
                        range: edit_range,
                        new_text: self.trigger_pattern().to_owned(),
                    }],
                );

                let edit_params = ApplyWorkspaceEditParams {
                    label: None,
                    edit: WorkspaceEdit {
                        changes: Some(changes),
                        ..Default::default()
                    },
                };

                sender.send_operation(edit_params.into()).await?;

                if !input.trim().is_empty() {
                    agent.cache.push(Message::new_user(input));

                    let trigger = Option::<ListenerTrigger>::None;
                    let mut response: ProviderStreamHandler = match t {
                        SingleLineVariant::QuickPrompt => {
                            agent.do_action(stream_completion, (), trigger).await?
                        }
                        SingleLineVariant::RagPrompt => {
                            agent
                                .do_action(stream_completion_with_rag, state_guard, trigger)
                                .await?
                        }
                        _ => unreachable!(),
                    };

                    sender
                        .send_work_done_report(Some("Got Stream Completion Handler"), None)
                        .await?;

                    let mut whole_message = String::new();
                    while let Some(status) = response.receive(agent).await {
                        warn!("starting inference response loop");
                        match status {
                            CompletionStreamStatus::Working(token) => {
                                warn!("got token: {}", token);
                                whole_message.push_str(&token);
                                sender.send_work_done_report(Some(&token), None).await?;
                            }
                            CompletionStreamStatus::Finished => {
                                warn!("finished");
                                sender.send_work_done_end(Some("Finished")).await?;
                            }
                        }
                    }

                    let message = ShowMessageParams {
                        typ: MessageType::INFO,
                        message: whole_message.clone(),
                    };

                    sender.send_operation(message.into()).await?;
                    self.try_change_state(uri.clone(), sender).await?;

                    return Ok(Some(format!(
                        "# User prompt\n{}\n# Assistant Response\n{}",
                        input, &whole_message,
                    )));
                }
            }

            SingleLineVariant::WalkProject | SingleLineVariant::LockDocIntoContext => {}
        }
        Ok(None)
    }
}
