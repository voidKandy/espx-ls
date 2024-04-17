mod actions;
pub mod cache;
mod echos;
pub mod error;

pub(self) use echos::*;

use lsp_server::RequestId;
use lsp_types::{
    ApplyWorkspaceEditParams, Diagnostic, DiagnosticSeverity, HoverContents,
    PublishDiagnosticsParams, Range, Url,
};
use tokio::sync::RwLockWriteGuard;

use crate::{
    handle::{operation_stream::BufferOpStreamSender, BufferOperation},
    state::GlobalState,
};

use self::{
    actions::{ActionBurn, ActionType},
    error::{BurnError, BurnResult},
};

#[derive(Debug, Clone)]
pub struct InBufferBurn {
    pub url: Url,
    pub burn: Burn,
}

#[derive(Debug, Clone)]
pub enum Burn {
    Action(ActionBurn),
    Echo(EchoBurn),
}

impl From<ActionBurn> for Burn {
    fn from(value: ActionBurn) -> Self {
        Self::Action(value)
    }
}

impl From<EchoBurn> for Burn {
    fn from(value: EchoBurn) -> Self {
        Self::Echo(value)
    }
}

impl Into<PublishDiagnosticsParams> for InBufferBurn {
    fn into(self) -> PublishDiagnosticsParams {
        PublishDiagnosticsParams {
            uri: self.url,
            diagnostics: vec![self.burn.diagnostic()],
            version: None,
        }
    }
}

impl Into<PublishDiagnosticsParams> for &InBufferBurn {
    fn into(self) -> PublishDiagnosticsParams {
        PublishDiagnosticsParams {
            uri: self.url.clone(),
            diagnostics: vec![self.burn.diagnostic()],
            version: None,
        }
    }
}

impl Burn {
    fn diagnostic(&self) -> Diagnostic {
        let severity = Some(DiagnosticSeverity::HINT);
        let (range, message) = match self {
            Self::Echo(echo) => (echo.range, String::new()),
            Self::Action(action) => {
                let message = match action.typ {
                    ActionType::IoPrompt => String::from("₳"),
                };
                (action.range, message)
            }
        };
        Diagnostic {
            range,
            severity,
            message,
            ..Default::default()
        }
    }

    pub fn hover_contents(&self) -> Option<HoverContents> {
        if let Burn::Echo(echo) = self {
            return Some(echo.hover_contents.clone());
        }
        None
    }

    pub fn range(&self) -> Range {
        match self {
            Self::Echo(echo) => echo.range,
            Self::Action(action) => action.range,
        }
    }
}

impl InBufferBurn {
    pub async fn goto_definition_action(
        mut self,
        request_id: RequestId,
        sender: &mut BufferOpStreamSender,
        state_guard: &mut RwLockWriteGuard<'_, GlobalState>,
    ) -> BurnResult<()> {
        match &mut self.burn {
            Burn::Action(ref mut action) => {
                if let Burn::Echo(echo) = action
                    .do_action(sender, self.url.clone(), state_guard)
                    .await?
                    .into()
                {
                    self.handle_action_echo(echo, sender).await?;
                    state_guard.cache.burns.save_burn(self)?;
                }
            }
            Burn::Echo(echo) => {
                echo.update_conversation_file(state_guard).await?;
                sender
                    .send_operation(BufferOperation::GotoFile {
                        id: request_id,
                        response: echo.goto_conversation_file(),
                    })
                    .await?;
            }
        }
        Ok(())
    }

    pub async fn handle_action_echo(
        &mut self,
        echo: EchoBurn,
        sender: &mut BufferOpStreamSender,
    ) -> BurnResult<()> {
        let edit_params = ApplyWorkspaceEditParams {
            label: None,
            edit: echo.workspace_edit(self.url.clone()),
        };
        sender.send_operation(edit_params.into()).await?;
        self.burn = Burn::Echo(echo);
        Ok(())
    }

    pub fn all_actions_on_document(text: &str, url: Url) -> Option<Vec<Self>> {
        let v = ActionType::parse_for_actions(text).into_iter().fold(
            vec![],
            |mut all_burns, action| {
                all_burns.push(Self {
                    url: url.to_owned(),
                    burn: Burn::Action(action),
                });
                all_burns
            },
        );
        match v.is_empty() {
            true => None,
            false => Some(v),
        }
    }
}
