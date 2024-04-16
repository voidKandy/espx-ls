mod actions;
pub mod cache;
mod echos;
pub mod error;

use crossbeam_channel::Sender;
pub(self) use echos::*;

use lsp_server::Message;
use lsp_types::{
    Diagnostic, DiagnosticSeverity, HoverContents, PublishDiagnosticsParams, Range, Url,
};

use crate::handle::BufferOperation;

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
        sender: Sender<Message>,
    ) -> BurnResult<(Sender<Message>, BufferOperation)> {
        unimplemented!()
        //     match &mut self.burn {
        //     Burn::Action(ref mut action) => {
        //         let ret = action.do_action(sender, self.url.clone()).await?;
        //         self.burn = Burn::Echo(ret.echo);
        //         return Ok((ret.sender, ret.operation));
        //
        //     }
        //     Burn::Echo(echo) => {
        //         echo.update_conversation_file()?;
        //         // return Ok((sender, BufferOperation::GotoFile { id: (), response: () }echo.goto_definition_response()))
        //         //
        //     }
        //     }
        // }
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
