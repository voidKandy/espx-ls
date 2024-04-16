pub mod diagnostics;
pub mod error;
pub mod notifications;
pub mod operation_stream;
pub mod requests;

use lsp_types::{
    ApplyWorkspaceEditParams, CodeActionResponse, GotoDefinitionResponse, HoverContents,
    ShowMessageParams,
};
pub use notifications::handle_notification;
pub use requests::handle_request;

use diagnostics::EspxDiagnostic;
use futures::{self, Stream, StreamExt};
use log::warn;
use lsp_server::{Message, RequestId};

use crate::handle::operation_stream::BufferOpStreamHandler;

use self::error::EspxLsHandleError;

pub type EspxLsResult<T> = Result<T, EspxLsHandleError>;

#[derive(Debug, Clone)]
pub enum BufferOperation {
    Diagnostics(EspxDiagnostic),
    ShowMessage(ShowMessageParams),
    // WorkspaceEdit(ApplyWorkspaceEditParams),
    // CodeActionExecute(EspxActionExecutor),
    GotoFile {
        id: RequestId,
        response: GotoDefinitionResponse,
    },
    HoverResponse {
        id: RequestId,
        contents: HoverContents,
    },
    // CodeActionRequest {
    //     id: RequestId,
    //     response: CodeActionResponse,
    // },
}

impl From<EspxDiagnostic> for BufferOperation {
    fn from(value: EspxDiagnostic) -> Self {
        Self::Diagnostics(value)
    }
}

pub fn handle_other(msg: Message) -> EspxLsResult<BufferOpStreamHandler> {
    warn!("unhandled message {:?}", msg);
    Ok(BufferOpStreamHandler::new())
}
