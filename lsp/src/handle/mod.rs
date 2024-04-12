pub mod diagnostics;
pub mod error;
pub mod notifications;
pub mod requests;
pub mod runes;

use lsp_types::{CodeActionResponse, GotoDefinitionResponse, HoverContents};
pub use notifications::handle_notification;
pub use requests::handle_request;

use diagnostics::EspxDiagnostic;
use log::warn;
use lsp_server::{Message, RequestId};

use self::{error::EspxLsHandleError, runes::EspxActionExecutor};

pub type EspxLsResult<T> = Result<T, EspxLsHandleError>;

#[derive(Debug)]
pub enum BufferOperation {
    Diagnostics(EspxDiagnostic),
    CodeActionExecute(EspxActionExecutor),
    GotoFile {
        id: RequestId,
        response: GotoDefinitionResponse,
    },
    HoverResponse {
        id: RequestId,
        contents: HoverContents,
    },
    CodeActionRequest {
        id: RequestId,
        response: CodeActionResponse,
    },
}

impl From<EspxDiagnostic> for BufferOperation {
    fn from(value: EspxDiagnostic) -> Self {
        Self::Diagnostics(value)
    }
}

pub fn handle_other(msg: Message) -> EspxLsResult<Option<BufferOperation>> {
    warn!("unhandled message {:?}", msg);
    Ok(None)
}
