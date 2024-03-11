pub mod notifications;
pub mod requests;

pub use notifications::handle_notification;
pub use requests::handle_request;

use crate::{actions::EspxActionExecutor, diagnostics::EspxDiagnostic};
use log::warn;
use lsp_server::{Message, RequestId};
use lsp_types::CodeActionResponse;

#[derive(Debug)]
pub enum EspxResult {
    Diagnostics(EspxDiagnostic),
    CodeActionExecute(EspxActionExecutor),
    CodeActionRequest {
        response: CodeActionResponse,
        id: RequestId,
    },
}

impl From<EspxDiagnostic> for EspxResult {
    fn from(value: EspxDiagnostic) -> Self {
        Self::Diagnostics(value)
    }
}

pub fn handle_other(msg: Message) -> Option<EspxResult> {
    warn!("unhandled message {:?}", msg);
    None
}
