use crate::{
    espx_env::{io_prompt_agent, CopilotAgent},
    parsing::get_prompt_and_position,
    text_store::{get_text_document, get_text_document_current},
};
use anyhow::anyhow;
use crossbeam_channel::Sender;
use espionox::environment::agent::memory::{MessageRole, ToMessage};
use lsp_server::{Message, Notification};
use lsp_types::{
    CodeAction, CodeActionParams, Command, ExecuteCommandParams, MessageType, Position,
    ShowMessageRequestParams, Url,
};
use serde_json::{json, Value};

pub enum EspxAction {
    PromptOnLine,
    LookAtMe,
}

#[derive(Debug)]
pub enum EspxActionExecutor {
    PromptOnLine { position: Position, prompt: String },
    LookAtMe { uri: Url },
}

pub enum EspxActionBuilder {
    PromptOnLine { position: Position, prompt: String },
    LookAtMe { uri: Url },
}

impl EspxAction {
    pub fn all_variants() -> Vec<EspxAction> {
        vec![EspxAction::LookAtMe, EspxAction::PromptOnLine]
    }
    pub fn try_from_params(&self, params: &CodeActionParams) -> Option<EspxActionBuilder> {
        match self {
            Self::LookAtMe => Some(EspxActionBuilder::LookAtMe {
                uri: params.text_document.uri.clone(),
            }),
            Self::PromptOnLine => {
                let uri = &params.text_document.uri;
                if let Some(text) = get_text_document_current(&uri) {
                    if let Some((prompt, pos)) = get_prompt_and_position(&text) {
                        return Some(EspxActionBuilder::PromptOnLine {
                            position: pos,
                            prompt,
                        });
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
            Self::LookAtMe { uri } => Some(vec![json!({"uri": uri.to_string()})]),
            Self::PromptOnLine { position, prompt } => {
                Some(vec![json!({"position": position, "prompt": prompt})])
            }
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
            Self::PromptOnLine { position, prompt } => {
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
                let response = io_prompt_agent(&prompt, CopilotAgent::Code).await?;
                sender.send(Message::Notification(Notification {
                    method: "window/showMessage".to_string(),
                    params: serde_json::to_value(ShowMessageRequestParams {
                        typ: MessageType::INFO,
                        message: String::from(response),
                        actions: None,
                    })?,
                }))?;
            }
            Self::LookAtMe { uri } => {
                let doc = get_text_document(&uri).ok_or(anyhow!("Could not get text document"))?;
                let message = doc.to_message(MessageRole::User);
                let response = io_prompt_agent(&message.content, CopilotAgent::Watcher).await?;
                sender.send(Message::Notification(Notification {
                    method: "window/showMessage".to_string(),
                    params: serde_json::to_value(ShowMessageRequestParams {
                        typ: MessageType::INFO,
                        message: String::from(response),
                        actions: None,
                    })?,
                }))?;
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
                if let Some(pos) = params
                    .arguments
                    .iter_mut()
                    .find_map(|arg| arg.as_object_mut()?.remove("position"))
                {
                    let position: Position = serde_json::from_value(pos)?;
                    let ex = EspxActionExecutor::PromptOnLine { position, prompt };
                    return Ok(ex);
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
                let ex = EspxActionExecutor::LookAtMe { uri };
                return Ok(ex);
            }
        }
        Err(anyhow!("No executor could be built"))
    }
}

impl Into<CodeAction> for EspxActionBuilder {
    fn into(self) -> CodeAction {
        match &self {
            &Self::PromptOnLine { .. } => {
                let title = "Prompt on Line".to_string();
                CodeAction {
                    title,
                    command: Some(self.command()),
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
