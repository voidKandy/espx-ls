pub mod diagnostics;
pub mod error;
pub mod notifications;
pub mod requests;
pub mod runes;

use lsp_types::{CodeActionResponse, HoverContents};
pub use notifications::handle_notification;
pub use requests::handle_request;

use diagnostics::EspxDiagnostic;
use log::warn;
use lsp_server::{Message, RequestId};

use self::{error::EspxHandleError, runes::EspxActionExecutor};

pub type EspxResult<T> = Result<T, EspxHandleError>;

#[derive(Debug)]
pub enum BufferOperation {
    Diagnostics(EspxDiagnostic),
    HoverResponse {
        contents: HoverContents,
        id: RequestId,
    },
    CodeActionExecute(EspxActionExecutor),
    CodeActionRequest {
        response: CodeActionResponse,
        id: RequestId,
    },
}

impl From<EspxDiagnostic> for BufferOperation {
    fn from(value: EspxDiagnostic) -> Self {
        Self::Diagnostics(value)
    }
}

pub fn handle_other(msg: Message) -> EspxResult<Option<BufferOperation>> {
    warn!("unhandled message {:?}", msg);
    Ok(None)
}
