use std::collections::HashMap;

use crate::{
    espx_env::{io_prompt_agent, prompt_from_file, CopilotAgent},
    parsing::{get_prompt_and_position, PREFIX},
    store::{get_text_document, get_text_document_current},
};
use anyhow::anyhow;
use crossbeam_channel::Sender;
use espionox::agents::memory::{MessageRole, ToMessage};
use log::{debug, error, info, warn};
use lsp_server::{Message, Notification};
use lsp_types::{
    CodeAction, CodeActionParams, Command, Diagnostic, DiagnosticSeverity, ExecuteCommandParams,
    MessageType, Position, PublishDiagnosticsParams, Range, ShowMessageRequestParams, TextEdit,
    Url, WorkspaceEdit,
};
use serde_json::{json, Value};

pub enum EspxAction {
    PromptOnLine,
    LookAtMe,
}

#[derive(Debug)]
pub enum EspxActionExecutor {
    PromptOnLine {
        uri: Url,
        new_text: String,
        line: u32,
        prompt: String,
    },
    LookAtMe {
        uri: Url,
        range: Range,
    },
}

pub enum EspxActionBuilder {
    PromptOnLine {
        uri: Url,
        new_text: String,
        line: u32,
        prompt: String,
    },
    LookAtMe {
        uri: Url,
        // range: Range,
    },
}

impl EspxAction {
    pub fn all_variants() -> Vec<EspxAction> {
        vec![EspxAction::LookAtMe, EspxAction::PromptOnLine]
    }
    pub fn try_from_params(&self, params: &CodeActionParams) -> Option<EspxActionBuilder> {
        match self {
            Self::LookAtMe => Some(EspxActionBuilder::LookAtMe {
                uri: params.text_document.uri.clone(),
                // range: params.range,
            }),
            Self::PromptOnLine => {
                let uri = &params.text_document.uri;
                if let Some(text) = get_text_document_current(&uri) {
                    if let Some((prompt, pos)) = get_prompt_and_position(&text) {
                        debug!(
                            "PROMPT ON LINE GOT TEXT: {} PROMPT: {} AND POSITION: {:?}",
                            text, prompt, pos
                        );
                        if params.range.end.line == pos.line || params.range.start.line == pos.line
                        {
                            let new_text = text
                                .lines()
                                .into_iter()
                                .nth(pos.line as usize)?
                                .split_once(PREFIX)?
                                .0
                                .to_owned();
                            debug!("PROMPT ON LINE GOT NEWTEXT: {} ", new_text);
                            let line = pos.line;
                            return Some(EspxActionBuilder::PromptOnLine {
                                uri: uri.clone(),
                                new_text,
                                line,
                                prompt,
                            });
                        }
                    }
                }
                None
            }
        }
    }
    pub fn command_id(&self) -> String {
        String::from(match self {
            Self::LookAtMe => "look_at_me",
            Self::PromptOnLine => "prompt_on_line",
        })
    }
}

impl EspxActionBuilder {
    fn builder(&self) -> EspxAction {
        match self {
            Self::LookAtMe { .. } => EspxAction::LookAtMe,
            Self::PromptOnLine { .. } => EspxAction::PromptOnLine,
        }
    }
    fn command_args(&self) -> Option<Vec<Value>> {
        match self {
            Self::LookAtMe { uri } => Some(vec![json!({
                // "range": range,
                "uri": uri.to_string()})]),
            Self::PromptOnLine {
                uri,
                new_text,
                line,
                prompt,
            } => Some(vec![json!({
                "uri": uri,
                "new_text": new_text, "line": line,
                "prompt": prompt})]),
        }
    }
    fn command(&self) -> Command {
        Command {
            title: self.builder().command_id(),
            command: self.builder().command_id(),
            arguments: self.command_args(),
        }
    }
}

impl EspxActionExecutor {
    pub async fn execute(self, sender: Sender<Message>) -> Result<Sender<Message>, anyhow::Error> {
        match self {
            Self::PromptOnLine { prompt, uri, .. } => {
                sender.send(Message::Notification(Notification {
                    method: "window/showMessage".to_string(),
                    params: serde_json::to_value(ShowMessageRequestParams {
                        typ: MessageType::INFO,
                        message: String::from("Prompting model..."),
                        actions: None,
                    })?,
                }))?;
                // Eventually the next thing to happen should be a diagnostic using the position to
                // show it in virtual text, but for now the response will be in a messagea
                match prompt_from_file(prompt).await {
                    Ok(response) => {
                        sender.send(Message::Notification(Notification {
                            method: "window/showMessage".to_string(),
                            params: serde_json::to_value(ShowMessageRequestParams {
                                typ: MessageType::INFO,
                                message: String::from(response),
                                actions: None,
                            })?,
                        }))?;
                    }
                    Err(err) => {
                        sender.send(Message::Notification(Notification {
                            method: "window/showMessage".to_string(),
                            params: serde_json::to_value(ShowMessageRequestParams {
                                typ: MessageType::ERROR,
                                message: err.to_string(),
                                actions: None,
                            })?,
                        }))?;
                    }
                }
            }

            Self::LookAtMe { range, uri } => {
                let doc = get_text_document(&uri).ok_or(anyhow!("Could not get text document"))?;
                let response = io_prompt_agent(doc, CopilotAgent::Watcher).await?;
                sender.send(Message::Notification(Notification {
                    method: "window/showMessage".to_string(),
                    params: serde_json::to_value(ShowMessageRequestParams {
                        typ: MessageType::INFO,
                        message: String::from("Looking at the codebase.."),
                        actions: None,
                    })?,
                }))?;

                sender.send(Message::Notification(Notification {
                    method: "textDocument/publishDiagnostics".to_string(),
                    params: serde_json::to_value(PublishDiagnosticsParams {
                        uri,
                        version: None,
                        diagnostics: vec![Diagnostic {
                            severity: Some(DiagnosticSeverity::HINT),
                            range,
                            message: response,
                            ..Default::default()
                        }],
                    })?,
                }))?;

                debug!("DIAGNOSTIC SHOULD HAVE SENT");
            }
        }
        Ok(sender)
    }
}

impl TryFrom<ExecuteCommandParams> for EspxActionExecutor {
    type Error = anyhow::Error;
    fn try_from(mut params: ExecuteCommandParams) -> Result<Self, Self::Error> {
        if params.command == EspxAction::PromptOnLine.command_id() {
            if let Some(prompt) = params
                .arguments
                .iter()
                .find_map(|arg| arg.as_object()?.get("prompt").map(|a| a.to_string()))
            {
                if let Some(l) = params
                    .arguments
                    .iter_mut()
                    .find_map(|arg| arg.as_object_mut()?.remove("line"))
                {
                    let line: u32 = serde_json::from_value(l)?;
                    if let Some(new_text) = params
                        .arguments
                        .iter_mut()
                        .find_map(|arg| arg.as_object_mut()?.remove("new_text"))
                    {
                        let new_text: String = serde_json::from_value(new_text)?;
                        if let Some(u) = params
                            .arguments
                            .iter_mut()
                            .find_map(|arg| arg.as_object_mut()?.remove("uri"))
                        {
                            let uri: Url = serde_json::from_value(u)?;
                            let ex = EspxActionExecutor::PromptOnLine {
                                uri,
                                new_text,
                                line,
                                prompt,
                            };
                            return Ok(ex);
                        }
                    }
                }
            }
        }

        if params.command == EspxAction::LookAtMe.command_id() {
            if let Some(uri) = params.arguments.iter_mut().find_map(|arg| {
                arg.as_object_mut()?.remove("uri").map(|a| {
                    let uri: Url = serde_json::from_value(a)
                        .expect("Failed to parse url from argument string");
                    uri
                })
            }) {
                // GET RANGE BY PARSING FOR CHANGES
                if let Some(range) = params.arguments.iter_mut().find_map(|arg| {
                    arg.as_object_mut()?.remove("range").map(|a| {
                        serde_json::from_value::<Range>(a)
                            .expect("Failed to parse url from argument string")
                    })
                }) {
                    let ex = EspxActionExecutor::LookAtMe { range, uri };
                    return Ok(ex);
                }
            }
        }
        Err(anyhow!("No executor could be built"))
    }
}

impl Into<CodeAction> for EspxActionBuilder {
    fn into(self) -> CodeAction {
        match &self {
            Self::PromptOnLine {
                uri,
                new_text,
                line,
                prompt,
            } => {
                let title = format!("[{}] : {:?}", prompt, line);
                let mut changes = HashMap::new();
                let range = Range {
                    end: Position {
                        line: line + 1,
                        character: 0,
                    },
                    start: Position {
                        line: *line,
                        character: 0,
                    },
                };
                let textedit = TextEdit {
                    range,
                    new_text: format!("{}\n", new_text),
                };
                changes.insert(uri.to_owned(), vec![textedit]);

                let edit = WorkspaceEdit {
                    changes: Some(changes),
                    ..Default::default()
                };
                CodeAction {
                    title,
                    command: Some(self.command()),
                    edit: Some(edit),
                    ..Default::default()
                }
            }
            &Self::LookAtMe { .. } => {
                let title = "Look At Me".to_string();
                CodeAction {
                    title,
                    command: Some(self.command()),
                    ..Default::default()
                }
            }
        }
    }
}
