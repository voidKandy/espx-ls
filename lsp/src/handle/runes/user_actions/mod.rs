// pub mod user_prompt;
use std::collections::HashMap;

use crate::{
    config::GLOBAL_CONFIG,
    espx_env::{
        agents::{get_inner_agent_handle, inner::InnerAgent},
        ENV_HANDLE,
    },
};
use espionox::{
    agents::memory::Message as EspxMessage,
    environment::{dispatch::EnvNotification, DispatchError, EnvError, EnvHandleError},
};
use lsp_types::{
    ApplyWorkspaceEditParams, Diagnostic, DiagnosticSeverity, MessageType, Position as LspPos,
    PublishDiagnosticsParams, Range, ShowMessageParams, TextEdit, Url, WorkspaceEdit,
};

use super::{generate_placeholder_for_doc, parsing::get_prompt_on_line, ActionRune};

#[derive(Debug, Clone)]
pub struct UserIoPrompt {
    url: Url,
    replacement_text: String,
    prompt: String,
    pos: LspPos,
}

impl ActionRune for UserIoPrompt {
    fn all_in_text(text: &str, url: &Url) -> Vec<Self> {
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
                        url: url.clone(),
                        replacement_text,
                        prompt,
                        pos,
                    })
                }
            }
        }

        action_vec
    }

    fn trigger_string() -> &'static str {
        &GLOBAL_CONFIG.user_actions.io_trigger
    }
    fn into_buffer_burn(
        self,
        edit: Option<&lsp_types::ApplyWorkspaceEditParams>,
        message: Option<&lsp_types::ShowMessageParams>,
    ) -> super::RuneBufferBurn {
        let line = self.pos.line;
        let placeholder = generate_placeholder_for_doc(&self.url);
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

        super::RuneBufferBurn {
            placeholder: (self.replacement_text, placeholder),
            diagnostic_params,
        }
    }
    async fn do_action(&self) -> anyhow::Result<super::DoActionReturn, super::error::RuneError> {
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
