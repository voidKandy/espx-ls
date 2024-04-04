pub mod diagnostics;
pub mod notifications;
pub mod requests;
pub mod runes;

pub use notifications::handle_notification;
pub use requests::handle_request;

use diagnostics::EspxDiagnostic;
use log::warn;
use lsp_server::Message;

#[derive(Debug)]
pub enum EspxResult {
    Diagnostics(EspxDiagnostic),
    // CodeActionExecute(UserAction),
    // CodeActionRequest {
    //     response: CodeActionResponse,
    //     id: RequestId,
    // },
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
