use std::collections::HashMap;

use crossbeam_channel::Sender;
use espionox::{
    agents::memory::Message as EspxMessage,
    environment::{dispatch::EnvNotification, DispatchError, EnvError, EnvHandleError},
};
use lsp_server::{Message, Notification, Request, RequestId};
use lsp_types::{
    ApplyWorkspaceEditParams, Diagnostic, DiagnosticSeverity, HoverContents, MarkupKind,
    MessageType, Position, PublishDiagnosticsParams, Range, ShowMessageParams, TextEdit, Url,
    WorkspaceEdit,
};

use crate::{
    config::GLOBAL_CONFIG,
    espx_env::{
        agents::{get_inner_agent_handle, inner::InnerAgent},
        ENV_HANDLE,
    },
    handle::BufferOperation,
    parsing::get_prompt_on_line,
};

use super::{error::BurnResult, Burn, EchoBurn};

/// Action Burns are parsed from the document
#[derive(Debug, Clone)]
pub(super) struct ActionBurn {
    pub(super) typ: ActionType,
    pub(super) range: Range,
    user_input: String,
    replacement_text: String,
}

#[derive(Debug, Clone)]
pub(super) enum ActionType {
    IoPrompt,
}

impl ActionType {
    fn all_variants() -> Vec<ActionType> {
        vec![ActionType::IoPrompt]
    }

    /// Gets trigger from GLOBAL_CONFIG, appends a whitespace
    fn trigger_string(&self) -> String {
        let actions_config = &GLOBAL_CONFIG.user_actions;
        format!(
            "{} ",
            match self {
                Self::IoPrompt => actions_config.io_trigger.to_owned(),
            }
        )
    }

    /// Parses document for all actions
    pub(super) fn parse_for_actions(text: &str, // , url: Url
    ) -> Vec<ActionBurn> {
        let mut action_vec = vec![];
        for typ in Self::all_variants().into_iter() {
            let trigger_string = typ.trigger_string();
            for (i, l) in text.lines().into_iter().enumerate() {
                if l.contains(&trigger_string) {
                    if let Some((replacement_text, prompt)) = get_prompt_on_line(l, &trigger_string)
                    {
                        log::info!("PARSED PROMPT: {}", prompt);
                        let start = Position {
                            line: i as u32,
                            character: (replacement_text.len() + trigger_string.len()) as u32,
                        };
                        let end = Position {
                            line: i as u32,
                            character: (replacement_text.len()
                                + trigger_string.len()
                                + prompt.len()) as u32,
                        };
                        action_vec.push(ActionBurn {
                            typ: typ.clone(),
                            // url: url.to_owned(),
                            replacement_text,
                            user_input: prompt,
                            range: Range { start, end },
                        })
                    }
                }
            }
        }
        action_vec
    }

    /// Notification sent to client when action is being done
    fn doing_action_notification(&self) -> Option<Notification> {
        match self {
            Self::IoPrompt => {
                let show_message = ShowMessageParams {
                    typ: MessageType::LOG,
                    message: String::from("Prompting Agent"),
                };
                Some(Notification {
                    method: "window/showMessage".to_string(),
                    params: serde_json::to_value(show_message)
                        .expect("Failed to serialize show_message"),
                })
            }
        }
    }
}

pub struct BurnDoActionReturn {
    pub sender: Sender<Message>,
    pub operation: BufferOperation,
    pub echo: EchoBurn,
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
                character: self.range.end.character,
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
        sender: Sender<Message>,
        url: Url,
    ) -> BurnResult<BurnDoActionReturn> {
        // if let Some(noti) = self.typ.doing_action_notification() {
        // sender.send(Message::Notification(noti))?;
        // }

        // sender.send(Message::Request(Request {
        //     id: RequestId::from(format!("Edit for prompt: {}", self.user_input)),
        //     method: "workspace/applyEdit".to_owned(),
        //     params: serde_json::to_value(ApplyWorkspaceEditParams {
        //         label: None,
        //         edit: self.workspace_edit(url.clone()),
        //     })?,
        // }))?;

        match self.typ {
            ActionType::IoPrompt => {
                let handle = get_inner_agent_handle(InnerAgent::Assistant).unwrap();

                let mut env_handle = ENV_HANDLE.get().unwrap().lock().unwrap();
                if !env_handle.is_running() {
                    let _ = env_handle.spawn();
                }

                let ticket = handle
                    .request_io_completion(EspxMessage::new_user(&self.user_input))
                    .await
                    .map_err(|err| {
                        EnvHandleError::from(EnvError::from(DispatchError::from(err)))
                    })?;

                let response: EnvNotification = env_handle.wait_for_notification(&ticket).await?;
                let response: &EspxMessage = response.extract_body().try_into()?;

                let message = ShowMessageParams {
                    typ: MessageType::INFO,
                    message: response.content.clone(),
                };

                let operation = BufferOperation::ShowMessage(message);

                let echo = {
                    let content = EchoBurn::generate_placeholder();
                    let range = Range {
                        start: Position {
                            line: self.range.start.line,
                            character: (self.replacement_text.len()
                                + self.typ.trigger_string().len())
                                as u32,
                        },
                        end: Position {
                            line: self.range.end.line,

                            character: (self.replacement_text.len()
                                + self.typ.trigger_string().len())
                                as u32
                                + 1,
                        },
                    };

                    let hover_contents = HoverContents::Markup(lsp_types::MarkupContent {
                        kind: MarkupKind::Markdown,
                        value: [
                            "# User prompt: ",
                            &self.user_input,
                            "# Assistant Response: ",
                            &response.content,
                        ]
                        .join("\n"),
                    });

                    EchoBurn {
                        content,
                        range,
                        hover_contents,
                    }
                };
                Ok(BurnDoActionReturn {
                    sender,
                    operation,
                    echo,
                })
            }
        }
    }
}
