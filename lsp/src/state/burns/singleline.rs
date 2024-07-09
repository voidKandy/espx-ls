use super::Burn;
use crate::{
    config::GLOBAL_CONFIG,
    handle::{buffer_operations::BufferOperation, error::HandleResult},
    state::{store::walk_dir, GlobalState},
};
use anyhow::anyhow;
use espionox::{
    agents::{actions::stream_completion, memory::Message, Agent},
    language_models::completions::streaming::{CompletionStreamStatus, ProviderStreamHandler},
};
use lsp_server::RequestId;
use lsp_types::{
    ApplyWorkspaceEditParams, GotoDefinitionResponse, HoverContents, Location, MarkupKind,
    MessageType, Position, Range, ShowMessageParams, TextEdit, Uri, WorkDoneProgress,
    WorkDoneProgressBegin, WorkspaceEdit,
};
use serde::{Deserialize, Serialize};
use std::{collections::HashMap, path::PathBuf, str::FromStr};
use tokio::sync::RwLockWriteGuard;
use tracing::{debug, warn};

#[derive(Debug, PartialEq, Eq)]
pub struct TextAndCharRange {
    pub text: String,
    pub start: u32,
    pub end: u32,
}

impl TextAndCharRange {
    // i dont love this
    /// returns None if neither are triggered, true if the trigger is user input
    fn user_input_is_triggered(
        position: Position,
        trigger_info: &TextAndCharRange,
        user_input_info_opt: &Option<TextAndCharRange>,
    ) -> Option<bool> {
        if trigger_info.start <= position.character && trigger_info.end >= position.character {
            Some(false)
        } else {
            user_input_info_opt.as_ref().and_then(|info| {
                Some(info.start <= position.character && info.end >= position.character)
            })
        }
    }
}

#[derive(Clone, Serialize, Deserialize, Debug)]
pub enum SingleLineBurn {
    QuickPrompt {
        hover_contents: Option<HoverContents>,
    },
    RagPrompt {
        hover_contents: Option<HoverContents>,
    },
    WalkProject {
        hover_contents: Option<HoverContents>,
    },
}

impl Burn for SingleLineBurn {
    fn all_variants() -> Vec<Self> {
        vec![
            Self::RagPrompt {
                hover_contents: None,
            },
            Self::QuickPrompt {
                hover_contents: None,
            },
            Self::WalkProject {
                hover_contents: None,
            },
        ]
    }

    fn trigger_string(&self) -> String {
        let actions_config = &GLOBAL_CONFIG.user_actions;
        match self {
            Self::QuickPrompt { .. } => actions_config.quick_prompt.to_owned(),
            Self::RagPrompt { .. } => actions_config.rag_prompt.to_owned(),
            Self::WalkProject { .. } => actions_config.walk_project.to_owned(),
        }
    }

    fn user_input_diagnostic(&self) -> Option<String> {
        match self {
            Self::RagPrompt { .. } => Some("Goto Def to RAGPrompt agent".to_owned()),
            Self::QuickPrompt { .. } => Some("Goto Def to QuickPrompt agent".to_owned()),
            Self::WalkProject { .. } => None,
        }
    }

    fn trigger_diagnostic(&self) -> Option<String> {
        match self {
            Self::RagPrompt { .. } => None,
            Self::QuickPrompt { .. } => None,
            Self::WalkProject { .. } => Some("Goto Def to trigger a directory walk".to_owned()),
        }
    }

    fn doing_action_notification(&self) -> Option<BufferOperation> {
        match self {
            Self::QuickPrompt { .. } => {
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
            Self::RagPrompt { .. } => {
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
            Self::WalkProject { .. } => {
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

    async fn activate(
        &mut self,
        uri: Uri,
        request_id: Option<RequestId>,
        position: Option<Position>,
        sender: &mut crate::handle::buffer_operations::BufferOpChannelSender,
        agent: &mut Agent,
        state_guard: &mut RwLockWriteGuard<'_, GlobalState>,
    ) -> HandleResult<()> {
        let request_id = request_id.ok_or(anyhow!(
            "request ID should be some when activating single line burns"
        ))?;
        let position = position.ok_or(anyhow!(
            "position should be some when activating single line burns"
        ))?;

        debug!(
            "activating single line burn on document: {:?} at position: {:?}",
            uri, position
        );

        let doc = state_guard
            .store
            .get_doc(&uri)
            .map_err(|e| anyhow!("Could not get document: {:?}", e))?;

        let (user_input_info_opt, trigger_info) =
            self.parse_for_user_input_and_trigger(position.line, &doc)?;

        let user_input_triggered_opt = TextAndCharRange::user_input_is_triggered(
            position,
            &trigger_info,
            &user_input_info_opt,
        );

        if let Some(trigger_is_user_input) = user_input_triggered_opt {
            if !trigger_is_user_input {
                match self {
                    SingleLineBurn::RagPrompt { .. } | SingleLineBurn::QuickPrompt { .. } => {
                        state_guard.update_conversation_file(&agent)?;
                    }
                    _ => {}
                }

                self.goto_definition_on_trigger(
                    request_id,
                    &position,
                    &trigger_info,
                    &user_input_info_opt,
                    uri,
                    sender,
                    agent,
                    state_guard,
                )
                .await?;
            } else {
                self.goto_definition_on_input(
                    request_id,
                    &position,
                    &trigger_info,
                    &user_input_info_opt,
                    uri,
                    sender,
                    agent,
                    state_guard,
                )
                .await?;
            }
        } else {
            warn!(
                "no trigger parsed for req: {:?}\npos: {:?}",
                request_id, position
            )
        }
        Ok(())
    }
}

impl SingleLineBurn {
    #[tracing::instrument(name = "save hover contents to burn")]
    pub fn save_hover_contents(&mut self, content_str: String) {
        if content_str.trim().is_empty() {
            warn!("passed an empty string to save hover contents");
            return;
        }
        let new_hover_content = HoverContents::Markup(lsp_types::MarkupContent {
            kind: MarkupKind::Markdown,
            value: content_str,
        });

        match self {
            Self::RagPrompt { hover_contents } => *hover_contents = Some(new_hover_content),
            Self::QuickPrompt { hover_contents } => *hover_contents = Some(new_hover_content),
            Self::WalkProject { hover_contents } => *hover_contents = Some(new_hover_content),
        }
    }

    pub fn get_hover_contents(&self) -> Option<&HoverContents> {
        match self {
            Self::RagPrompt { hover_contents } => hover_contents.as_ref(),
            Self::QuickPrompt { hover_contents } => hover_contents.as_ref(),
            Self::WalkProject { hover_contents } => hover_contents.as_ref(),
        }
    }

    pub fn echo_content(&self) -> &str {
        match self {
            Self::RagPrompt { .. } => "⧗",
            Self::QuickPrompt { .. } => "⚑",
            Self::WalkProject { .. } => "⧉",
        }
    }

    #[allow(unused)]
    pub async fn goto_definition_on_trigger(
        &mut self,
        request_id: RequestId,
        position: &Position,
        trigger_info: &TextAndCharRange,
        user_input_info_opt: &Option<TextAndCharRange>,
        uri: Uri,
        sender: &mut crate::handle::buffer_operations::BufferOpChannelSender,
        agent: &mut Agent,
        state_guard: &mut RwLockWriteGuard<'_, GlobalState>,
    ) -> anyhow::Result<()> {
        debug!("activating burn on trigger: {:?}", self);
        match self {
            Self::QuickPrompt { .. } | Self::RagPrompt { .. } => {
                let path = &GLOBAL_CONFIG.paths.conversation_file_path;
                let path_str = format!("file:///{}", path.display().to_string());
                let op = BufferOperation::GotoFile {
                    id: request_id,
                    response: GotoDefinitionResponse::Scalar(Location {
                        uri: Uri::from_str(&path_str)?,
                        range: Range::default(),
                    }),
                };
                sender.send_operation(op).await?;
            }
            Self::WalkProject { .. } => {
                let edit_range = Range {
                    start: Position {
                        line: position.line as u32,
                        character: trigger_info.start as u32,
                    },
                    end: Position {
                        line: position.line as u32,
                        character: trigger_info.end,
                    },
                };

                let mut changes = HashMap::new();
                changes.insert(
                    uri,
                    vec![TextEdit {
                        range: edit_range,
                        new_text: self.echo_content().to_owned(),
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

                let docs = walk_dir(PathBuf::from("."))
                    .map_err(|err| anyhow!("error walking dir: {:?}", err))?;
                debug!("GOT DOCS: {:?}", docs);
                let mut update_counter = 0;
                for (i, (path, text)) in docs.iter().enumerate() {
                    sender
                        .send_work_done_report(
                            Some(&format!("Adding {} to memory...", path.display())),
                            Some((i as f32 / docs.len() as f32 * 100.0) as u32),
                        )
                        .await?;

                    let uri = Uri::from_str(&format!("file:///{}", path.display().to_string()))
                        .expect("Failed to build uri");
                    state_guard.store.update_doc(&text, uri);
                    update_counter += 1;
                }

                sender.send_work_done_end(None).await?;

                self.save_hover_contents(format!(
                    "Finished walking project, added {:?} docs to database.",
                    update_counter
                ));
            }
        }
        Ok(())
    }

    #[allow(unused)]
    pub async fn goto_definition_on_input(
        &mut self,
        request_id: RequestId,
        position: &Position,
        trigger_info: &TextAndCharRange,
        user_input_info_opt: &Option<TextAndCharRange>,
        uri: Uri,
        sender: &mut crate::handle::buffer_operations::BufferOpChannelSender,
        agent: &mut Agent,
        state_guard: &mut RwLockWriteGuard<'_, GlobalState>,
    ) -> anyhow::Result<()> {
        debug!("activating burn on user input: {:?}", self);
        if let Some(op) = self.doing_action_notification() {
            sender.send_operation(op).await?;
        }

        match &self {
            t @ Self::QuickPrompt { .. } | t @ Self::RagPrompt { .. } => {
                let user_input_info = user_input_info_opt
                    .as_ref()
                    .expect("user info option should never be none with prompt variants");
                let edit_range = Range {
                    start: Position {
                        line: position.line as u32,
                        character: trigger_info.start as u32,
                    },
                    end: Position {
                        line: position.line as u32,
                        character: user_input_info.end,
                    },
                };

                let mut changes = HashMap::new();
                changes.insert(
                    uri,
                    vec![TextEdit {
                        range: edit_range,
                        new_text: self.echo_content().to_owned(),
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

                if let Self::RagPrompt { .. } = t {
                    // state_guard
                    //     .espx_env
                    //     .updater
                    //     .inner_write_lock()?
                    //     .refresh_update_with_cache(&state_guard.store)
                    //     .await?;
                }
                if !user_input_info.text.trim().is_empty() {
                    agent.cache.push(Message::new_user(&user_input_info.text));

                    let trigger = if let Self::RagPrompt { .. } = *t {
                        Some("updater")
                    } else {
                        None
                    };
                    let mut response: ProviderStreamHandler =
                        agent.do_action(stream_completion, (), trigger).await?;

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

                    self.save_hover_contents(format!(
                        r#"
                        # User prompt: 
                        {}
                        # Assistant Response: 
                        {}"
                        "#,
                        &whole_message, &user_input_info.text,
                    ));
                }
            }

            Self::WalkProject { .. } => {}
        }

        Ok(())
    }

    pub fn parse_for_user_input_and_trigger(
        &self,
        line_no: u32,
        text: &str,
    ) -> anyhow::Result<(Option<TextAndCharRange>, TextAndCharRange)> {
        let line = text.lines().nth(line_no as usize).ok_or(anyhow!(
            "text has incorrect amount of lines. Expected at least {}",
            line_no + 1,
        ))?;

        if line.lines().count() != 1 {
            return Err(anyhow!(
                "text has incorrect amount of lines. Expected {} Got {}",
                1,
                text.lines().count()
            ));
        }
        let initial_whitespace_len = line
            .char_indices()
            .find(|(_, ch)| !ch.is_whitespace())
            .ok_or(anyhow!("the entire input string is whitespace"))?
            .0;
        let chunks: Vec<&str> = line.split_whitespace().collect();
        debug!("chunks: {:?}", chunks);
        let trigger = chunks
            .get(1)
            .ok_or(anyhow!("not enough whitespace separated chunks in line"))?;
        if trigger != &self.echo_content() && trigger != &self.trigger_string() {
            return Err(anyhow!(
                "[{}] is not a valid trigger. Accepts [{}] and [{}]",
                trigger,
                self.echo_content(),
                self.trigger_string()
            ));
        }

        let uinput = match &self {
            Self::QuickPrompt { .. } | Self::RagPrompt { .. } => {
                let user_input = chunks[2..].join(" ");
                let start = (initial_whitespace_len
                    + chunks[0].chars().count()
                    + trigger.chars().count()
                    + 2) as u32;
                let end = start + user_input.chars().count() as u32;
                Some(TextAndCharRange {
                    text: user_input,
                    start,
                    end,
                })
            }
            _ => None,
        };

        let start = (initial_whitespace_len + chunks[0].chars().count() + 1) as u32;
        let end = start + trigger.chars().count() as u32;

        let trig = TextAndCharRange {
            text: trigger.to_string(),
            start,
            end,
        };

        return Ok((uinput, trig));
    }
}

mod tests {
    use lsp_types::Position;

    use crate::{
        error::init_test_tracing,
        state::burns::singleline::{SingleLineBurn, TextAndCharRange},
    };

    #[test]
    fn correctly_differentiates_positional_trigger() {
        let burn = SingleLineBurn::QuickPrompt {
            hover_contents: None,
        };
        let input = r#"
        // also not
        // #$ helllo
        // not user input
        "#;

        let user_input_activation_position = Position {
            line: 2,
            character: 14,
        };
        let trigger_activation_position = Position {
            line: 2,
            character: 12,
        };

        let (user_input_info_opt, trigger_info) =
            burn.parse_for_user_input_and_trigger(2, &input).unwrap();
        assert!(TextAndCharRange::user_input_is_triggered(
            user_input_activation_position,
            &trigger_info,
            &user_input_info_opt,
        )
        .unwrap());
        assert!(!TextAndCharRange::user_input_is_triggered(
            trigger_activation_position,
            &trigger_info,
            &user_input_info_opt,
        )
        .unwrap());
    }

    #[test]
    fn correctly_parses_user_input() {
        init_test_tracing();
        let input = r#"
        this is not input
        // #$# this is user input 
        this is not 
        // ⚑ this is also user input"#;
        let output = SingleLineBurn::RagPrompt {
            hover_contents: None,
        }
        .parse_for_user_input_and_trigger(2, &input)
        .unwrap();
        let expected_user_text_char = Some(TextAndCharRange {
            text: "this is user input".to_string(),
            start: 15,
            end: 33,
        });
        let expected_trig_text_char = TextAndCharRange {
            text: "#$#".to_string(),
            start: 11,
            end: 14,
        };
        assert_eq!(output, (expected_user_text_char, expected_trig_text_char));

        let output = SingleLineBurn::QuickPrompt {
            hover_contents: None,
        }
        .parse_for_user_input_and_trigger(4, &input)
        .unwrap();
        let expected_user_text_char = Some(TextAndCharRange {
            text: "this is also user input".to_string(),
            start: 13,
            end: 36,
        });
        let expected_trig_text_char = TextAndCharRange {
            text: "⚑".to_string(),
            start: 11,
            end: 12,
        };
        assert_eq!(output, (expected_user_text_char, expected_trig_text_char));
    }
}
