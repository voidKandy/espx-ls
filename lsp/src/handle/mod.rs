pub mod diagnostics;
pub mod notifications;
pub mod requests;
pub mod runes;

use lsp_types::CodeActionResponse;
pub use notifications::handle_notification;
pub use requests::handle_request;

use diagnostics::EspxDiagnostic;
use log::warn;
use lsp_server::{Message, RequestId};

use self::runes::{ActionRune, ActionRuneExecutor};

#[derive(Debug)]
pub enum EspxResult {
    Diagnostics(EspxDiagnostic),
    CodeActionExecute(ActionRuneExecutor),
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
