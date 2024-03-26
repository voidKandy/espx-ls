use std::collections::HashMap;

use crate::{
    cache::GLOBAL_CACHE,
    database::{chunks::chunk_vec_content, DB},
    espx_env::{
        agents::{get_inner_agent_handle, inner::InnerAgent},
        ENV_HANDLE,
    },
    parsing::{get_all_prompts_and_positions, PREFIX},
};

use anyhow::anyhow;
use crossbeam_channel::Sender;
use espionox::agents::memory::Message as EspxMessage;
use log::debug;
use lsp_server::{Message, Notification};
use lsp_types::{
    CodeAction, CodeActionParams, Command, ExecuteCommandParams, MessageType, Position, Range,
    ShowMessageRequestParams, TextEdit, Url, WorkspaceEdit,
};
use serde_json::{json, Value};

#[derive(Debug)]
pub enum EspxAction {
    PromptOnLine,
}

#[derive(Debug)]
pub enum EspxActionExecutor {
    PromptOnLine {
        uri: Url,
        new_text: String,
        line: u32,
        prompt: String,
    },
}

pub enum EspxActionBuilder {
    PromptOnLine {
        uri: Url,
        new_text: String,
        line: u32,
        prompt: String,
    },
}

impl EspxAction {
    pub fn all_variants() -> Vec<EspxAction> {
        vec![EspxAction::PromptOnLine]
    }
    pub async fn try_from_params(
        &self,
        params: &CodeActionParams,
    ) -> Option<Vec<EspxActionBuilder>> {
        match self {
            Self::PromptOnLine => {
                let uri = &params.text_document.uri;
                if params.range.end.line == params.range.start.line {
                    if let Some(text) = GLOBAL_CACHE.write().unwrap().get_doc(&uri) {
                        return EspxActionBuilder::all_from_text_doc(&text, uri.clone());
                    }

                    if let Some(chunks) = DB.read().unwrap().get_chunks_by_url(&uri).await.ok() {
                        let text = chunk_vec_content(&chunks);
                        GLOBAL_CACHE.write().unwrap().update_doc(&text, uri.clone());
                        return EspxActionBuilder::all_from_text_doc(&text, uri.clone());
                    }
                }
            }
        }
        None
    }

    pub fn command_id(&self) -> String {
        String::from(match self {
            Self::PromptOnLine => "prompt_on_line",
        })
    }
}

impl EspxActionBuilder {
    fn builder(&self) -> EspxAction {
        match self {
            Self::PromptOnLine { .. } => EspxAction::PromptOnLine,
        }
    }
    fn command_args(&self) -> Option<Vec<Value>> {
        match self {
            Self::PromptOnLine {
                uri,
                new_text,
                line,
                prompt,
            } => Some(vec![json!({
                "uri": uri,
                "new_text": new_text, "line": line ,
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

    pub fn all_from_text_doc(text: &str, uri: Url) -> Option<Vec<Self>> {
        let mut all = vec![];
        let prompt_pos_vec = get_all_prompts_and_positions(&text);
        for (prompt, pos) in prompt_pos_vec.into_iter() {
            debug!(
                "FOUND TEXT: {} PROMPT: {} AND POSITION: {:?}",
                text, prompt, pos
            );
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
                all.push(EspxActionBuilder::PromptOnLine {
                    uri: uri.clone(),
                    new_text,
                    line,
                    prompt,
                });
            }
        }
        if all.is_empty() {
            return None;
        }
        Some(all)
    }
}

impl EspxActionExecutor {
    pub async fn execute(self, sender: Sender<Message>) -> Result<Sender<Message>, anyhow::Error> {
        match self {
            Self::PromptOnLine { prompt, .. } => {
                sender.send(Message::Notification(Notification {
                    method: "window/showMessage".to_string(),
                    params: serde_json::to_value(ShowMessageRequestParams {
                        typ: MessageType::INFO,
                        message: String::from("Prompting model..."),
                        actions: None,
                    })?,
                }))?;

                let handle = get_inner_agent_handle(InnerAgent::Assistant).unwrap();

                let mut env_handle = ENV_HANDLE.get().unwrap().lock().unwrap();
                if !env_handle.is_running() {
                    let _ = env_handle.spawn();
                }

                let ticket = handle
                    .request_io_completion(EspxMessage::new_user(&prompt))
                    .await?;
                match env_handle.wait_for_notification(&ticket).await {
                    // Eventually the next thing to happen should be a diagnostic using the position to
                    // show it in virtual text, but for now the response will be in a messagea
                    Ok(response) => {
                        let response: &EspxMessage = response.extract_body().try_into()?;
                        sender.send(Message::Notification(Notification {
                            method: "window/showMessage".to_string(),
                            params: serde_json::to_value(ShowMessageRequestParams {
                                typ: MessageType::INFO,
                                message: response.content.to_owned(),
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
                        line: *line + 1,
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
        }
    }
}
