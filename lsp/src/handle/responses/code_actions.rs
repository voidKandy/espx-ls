use std::collections::HashMap;

use crate::{
    cache::GLOBAL_CACHE,
    config::GLOBAL_CONFIG,
    espx_env::{
        agents::{get_inner_agent_handle, inner::InnerAgent},
        ENV_HANDLE,
    },
    parsing::UserAction,
};

use anyhow::anyhow;
use crossbeam_channel::Sender;
use espionox::agents::memory::Message as EspxMessage;
use lsp_server::{Message, Notification};
use lsp_types::{
    CodeAction, CodeActionParams, Command, ExecuteCommandParams, MessageType, Position, Range,
    ShowMessageRequestParams, TextEdit, Url, WorkspaceEdit,
};
use serde_json::{json, Value};

#[derive(Debug)]
pub enum EspxCodeActionVariant {
    PromptOnLine,
}

pub struct EspxCodeAction {
    url: Url,
    executor: EspxCodeActionExecutor,
    builder: EspxCodeActionBuilder,
}

#[derive(Debug)]
pub enum EspxCodeActionExecutor {
    PromptOnLine {
        uri: Url,
        rune: String,
        line: u32,
        prompt: String,
    },
}

pub enum EspxCodeActionBuilder {
    PromptOnLine {
        uri: Url,
        rune: String,
        line: u32,
        prompt: String,
    },
}

struct ActionBuilderVec(Vec<EspxCodeActionBuilder>);

impl From<Vec<EspxCodeActionBuilder>> for ActionBuilderVec {
    fn from(value: Vec<EspxCodeActionBuilder>) -> Self {
        Self(value)
    }
}

impl Into<Vec<EspxCodeActionBuilder>> for ActionBuilderVec {
    fn into(self) -> Vec<EspxCodeActionBuilder> {
        self.0
    }
}
impl TryFrom<(&CodeActionParams, &EspxCodeActionVariant)> for ActionBuilderVec {
    type Error = anyhow::Error;
    fn try_from(
        (params, variant): (&CodeActionParams, &EspxCodeActionVariant),
    ) -> Result<Self, Self::Error> {
        match variant {
            EspxCodeActionVariant::PromptOnLine => {
                let uri = &params.text_document.uri;
                if params.range.end.line == params.range.start.line {
                    if let Some(text) = GLOBAL_CACHE.write().unwrap().lru.get_doc(&uri) {
                        let all_builders =
                            EspxCodeActionBuilder::all_from_text_doc(&text, uri.clone())
                                .ok_or(anyhow!("Failed to get any builders from text document"))?;

                        return Ok(all_builders.into());
                    }

                    // Maybe use custom errors and handle an error where global cache doesn't
                    // contain doc by querying the database and then retrying

                    // if let Some(chunks) = DB.read().unwrap().get_chunks_by_url(&uri).await.ok() {
                    //     let text = chunk_vec_content(&chunks);
                    //     GLOBAL_CACHE
                    //         .write()
                    //         .unwrap()
                    //         .lru
                    //         .update_doc(&text, uri.clone());
                    //     return EspxCodeActionBuilder::all_from_text_doc(&text, uri.clone()).ok_or(
                    //         Err(anyhow!("Failed to get any builders from text document")),
                    //     );
                    // }
                }
            }
        }
        Err(anyhow!("Could not build action vec"))
    }
}

impl From<(UserAction, Url)> for EspxCodeActionBuilder {
    fn from((action, uri): (UserAction, Url)) -> Self {
        // RUNE LOGIC NEEDS TO BE WRITTEN
        let rune = String::from("$%");
        match action {
            UserAction::IoPrompt(params) => Self::PromptOnLine {
                uri,
                rune: format!("{} {}", params.replacement_text, rune),
                line: params.pos.line,
                prompt: params.prompt.to_owned(),
            },
        }
    }
}

impl EspxCodeActionVariant {
    pub fn all_variants() -> Vec<EspxCodeActionVariant> {
        vec![EspxCodeActionVariant::PromptOnLine]
    }

    pub fn command_id(&self) -> String {
        String::from(match self {
            Self::PromptOnLine => "prompt_on_line",
        })
    }
}

impl EspxCodeActionBuilder {
    fn action_variant(&self) -> EspxCodeActionVariant {
        match self {
            Self::PromptOnLine { .. } => EspxCodeActionVariant::PromptOnLine,
        }
    }

    fn command_args(&self) -> Option<Vec<Value>> {
        match self {
            Self::PromptOnLine {
                uri,
                rune,
                line,
                prompt,
            } => Some(vec![json!({
                "uri": uri,
                "new_text": rune,
                "line": line ,
                "prompt": prompt})]),
        }
    }

    fn command(&self) -> Command {
        Command {
            title: self.action_variant().command_id(),
            command: self.action_variant().command_id(),
            arguments: self.command_args(),
        }
    }

    // Later implement this to return a custom error which can be handled by updating the cache
    // with the databse
    pub fn all_from_lsp_params(
        params: &CodeActionParams,
        variant: &EspxCodeActionVariant,
    ) -> Option<Vec<Self>> {
        if let Some(vec) = ActionBuilderVec::try_from((params, variant)).ok() {
            return Some(vec.into());
        }
        None
    }

    pub fn all_from_text_doc(text: &str, uri: Url) -> Option<Vec<Self>> {
        let mut all_builders = vec![];
        let config = &GLOBAL_CONFIG;

        let line_actions: Vec<UserAction> =
            UserAction::all_actions_in_text(&config.user_actions, &text);

        for action in line_actions {
            all_builders.push(EspxCodeActionBuilder::from((action, uri.clone())));
        }

        match all_builders.is_empty() {
            true => None,
            false => Some(all_builders),
        }
    }
}

impl EspxCodeActionExecutor {
    fn lsp_init_execution_message(&self) -> Result<Notification, serde_json::Error> {
        let method = "window/showMessage".to_string();
        match self {
            Self::PromptOnLine {
                uri,
                rune,
                line,
                prompt,
            } => {
                let params = serde_json::to_value(ShowMessageRequestParams {
                    typ: MessageType::INFO,
                    message: String::from("Prompting model..."),
                    actions: None,
                })?;

                Ok(Notification { method, params })
            }
        }
    }

    pub async fn execute(self, sender: Sender<Message>) -> Result<Sender<Message>, anyhow::Error> {
        let init_execute_message = self.lsp_init_execution_message()?;
        sender.send(Message::Notification(init_execute_message))?;

        match self {
            Self::PromptOnLine { prompt, .. } => {
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

impl TryFrom<ExecuteCommandParams> for EspxCodeActionExecutor {
    type Error = anyhow::Error;
    fn try_from(mut params: ExecuteCommandParams) -> Result<Self, Self::Error> {
        if params.command == EspxCodeActionVariant::PromptOnLine.command_id() {
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
                        let rune: String = serde_json::from_value(new_text)?;
                        if let Some(u) = params
                            .arguments
                            .iter_mut()
                            .find_map(|arg| arg.as_object_mut()?.remove("uri"))
                        {
                            let uri: Url = serde_json::from_value(u)?;
                            let ex = EspxCodeActionExecutor::PromptOnLine {
                                uri,
                                rune,
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

impl Into<CodeAction> for EspxCodeActionBuilder {
    fn into(self) -> CodeAction {
        match &self {
            Self::PromptOnLine {
                uri,
                rune,
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
                    new_text: format!("{}\n", rune),
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
