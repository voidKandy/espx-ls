use std::collections::HashMap;

use crate::{
    cache::GLOBAL_CACHE,
    config::GLOBAL_CONFIG,
    espx_env::{
        agents::{get_inner_agent_handle, inner::InnerAgent},
        ENV_HANDLE,
    },
};
use anyhow::anyhow;
use espionox::{
    agents::memory::Message as EspxMessage,
    environment::{dispatch::EnvNotification, DispatchError, EnvError, EnvHandleError},
};
use lsp_types::{
    ApplyWorkspaceEditParams, CodeAction, CodeActionParams, Diagnostic, DiagnosticSeverity,
    ExecuteCommandParams, MessageType, Position as LspPos, PublishDiagnosticsParams, Range,
    ShowMessageParams, TextEdit, Url, WorkspaceEdit,
};
use serde_json::{json, Value};

use super::{
    error::RuneError, parsing::get_prompt_on_line, ActionRune, DoActionReturn, RuneBufferBurn,
    ToCodeAction,
};

#[derive(Debug, Clone)]
pub struct UserIoPrompt {
    url: Url,
    replacement_text: String,
    prompt: String,
    pos: LspPos,
}

impl ToCodeAction for UserIoPrompt {
    fn command_id() -> String {
        String::from("prompt_on_line")
    }

    fn title(&self) -> String {
        format!("[{}] : {:?}", self.prompt, self.pos.line)
    }

    fn command_args(&self) -> Option<Vec<Value>> {
        Some(vec![json!({
                "uri": self.url,
            "new_text": self.replacement_text,
                "line": self.pos.line ,
                "prompt": self.prompt})])
    }

    fn workspace_edit(&self) -> Option<WorkspaceEdit> {
        let line = self.pos.line;
        let mut changes = HashMap::new();
        let range = Range {
            end: LspPos {
                line: line + 1,
                character: 0,
            },
            start: LspPos { line, character: 0 },
        };

        let textedit = TextEdit {
            range,
            new_text: format!("{}\n", self.replacement_text),
        };

        changes.insert(self.url.to_owned(), vec![textedit]);

        let edit = WorkspaceEdit {
            changes: Some(changes),
            ..Default::default()
        };
        Some(edit)
    }

    fn to_code_action(&self) -> CodeAction {
        CodeAction {
            title: self.title(),
            command: Some(self.command()),
            edit: self.workspace_edit(),
            ..Default::default()
        }
    }
}

impl ActionRune for UserIoPrompt {
    fn all_from_action_params(params: CodeActionParams) -> Vec<Self> {
        // NEED A WAY TO HANDLE WHEN LRU DOESN'T HAVE DOC
        // Also needing a write lock for a read is kinda lame..
        let text = GLOBAL_CACHE
            .write()
            .unwrap()
            .lru
            .get_doc(&params.text_document.uri)
            .expect("Couldn't get doc from LRU");
        let mut action_vec = vec![];

        for (i, l) in text.lines().into_iter().enumerate() {
            if l.contains(Self::trigger_string()) {
                if let Some((replacement_text, prompt)) =
                    get_prompt_on_line(l, Self::trigger_string())
                {
                    let pos = LspPos {
                        line: i as u32,
                        character: prompt.len() as u32,
                    };
                    action_vec.push(Self {
                        url: params.text_document.uri.to_owned(),
                        replacement_text,
                        prompt,
                        pos,
                    })
                }
            }
        }

        action_vec
    }

    fn try_from_execute_command_params(
        mut params: ExecuteCommandParams,
    ) -> anyhow::Result<Self, RuneError> {
        if params.command == Self::command_id() {
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
                        let replacement_text: String = serde_json::from_value(new_text)?;
                        if let Some(u) = params
                            .arguments
                            .iter_mut()
                            .find_map(|arg| arg.as_object_mut()?.remove("uri"))
                        {
                            let url: Url = serde_json::from_value(u)?;
                            let ex = Self {
                                url,
                                pos: LspPos {
                                    line,
                                    character: (replacement_text.len() - 1) as u32,
                                },
                                replacement_text,
                                prompt,
                            };
                            return Ok(ex);
                        }
                    }
                }
            }
        }
        Err(RuneError::Undefined(anyhow!(
            "Execute command params command name doesn't match"
        )))
    }

    fn trigger_string() -> &'static str {
        &GLOBAL_CONFIG.user_actions.io_trigger
    }

    fn into_executor(
        self,
        do_action_return: DoActionReturn,
    ) -> Result<super::ActionRuneExecutor, RuneError> {
        let line = self.pos.line;
        let placeholder = RuneBufferBurn::generate_placeholder();
        let range: lsp_types::Range = lsp_types::Range {
            start: LspPos { line, character: 0 },
            end: LspPos { line, character: 0 },
        };
        let diagnostic = Diagnostic {
            range,
            severity: Some(DiagnosticSeverity::HINT),
            message: self.prompt,
            ..Default::default()
        };

        let diagnostic_params = PublishDiagnosticsParams {
            uri: self.url,
            diagnostics: vec![diagnostic],
            version: None,
        };

        let burn = super::RuneBufferBurn {
            placeholder: (self.replacement_text, placeholder),
            diagnostic_params,
        };

        Ok(super::ActionRuneExecutor {
            burn,
            workspace_edit: do_action_return.0,
            message: do_action_return.1,
        })
    }

    async fn do_action(&self) -> Result<super::DoActionReturn, RuneError> {
        let handle = get_inner_agent_handle(InnerAgent::Assistant).unwrap();

        let mut env_handle = ENV_HANDLE.get().unwrap().lock().unwrap();
        if !env_handle.is_running() {
            let _ = env_handle.spawn();
        }

        let ticket = handle
            .request_io_completion(EspxMessage::new_user(&self.prompt))
            .await
            .map_err(|err| EnvHandleError::from(EnvError::from(DispatchError::from(err))))?;

        let response: EnvNotification = env_handle.wait_for_notification(&ticket).await?;
        let response: &EspxMessage = response.extract_body().try_into()?;

        let message = ShowMessageParams {
            typ: MessageType::INFO,
            message: response.content.clone(),
        };

        let mut changes = HashMap::new();
        let line = self.pos.line;
        let range = Range {
            end: LspPos {
                line: line + 1,
                character: 0,
            },
            start: LspPos { line, character: 0 },
        };

        let textedit = TextEdit {
            range,
            new_text: format!("{}\n", self.replacement_text),
        };

        changes.insert(self.url.clone(), vec![textedit]);

        let edit = WorkspaceEdit {
            changes: Some(changes),
            ..Default::default()
        };

        let edit_params = ApplyWorkspaceEditParams {
            label: Some(format!("Remove {} on line {}", self.prompt, self.pos.line)),
            edit,
        };

        Ok((Some(edit_params), Some(message)))
    }
}
