mod actions;
pub mod cache;
mod echos;
pub mod error;

pub(self) use echos::*;

use lsp_types::{
    Diagnostic, DiagnosticSeverity, HoverContents, PublishDiagnosticsParams, Range, Url,
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
    // range: Range,
    pub url: Url,
    // content: String,
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
        &mut self,
        sender: &mut BufferOpStreamSender,
        state_guard: &mut RwLockWriteGuard<'_, GlobalState>,
    ) -> BurnResult<()> {
        match &mut self.burn {
            Burn::Action(ref mut action) => {
                self.burn = action
                    .do_action(sender, self.url.clone(), state_guard)
                    .await?
                    .into();
            }
            Burn::Echo(echo) => {
                echo.update_conversation_file(state_guard).await?;
            }
        }
        Ok(())
    }

    pub fn all_on_document(text: &str, url: Url) -> Vec<Self> {
        ActionType::parse_for_actions(text)
            .into_iter()
            .fold(vec![], |mut all_burns, action| {
                all_burns.push(Self {
                    url: url.to_owned(),
                    burn: Burn::Action(action),
                });
                all_burns
            })
    }
}
