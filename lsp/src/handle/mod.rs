pub mod notifications;
pub mod requests;
pub mod responses;

pub use notifications::handle_notification;
pub use requests::handle_request;

use log::warn;
use lsp_server::{Message, RequestId};
use lsp_types::CodeActionResponse;
use responses::{code_actions::EspxCodeActionExecutor, diagnostics::EspxDiagnostic};

#[derive(Debug)]
pub enum EspxResult {
    Diagnostics(EspxDiagnostic),
    CodeActionExecute(EspxCodeActionExecutor),
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
