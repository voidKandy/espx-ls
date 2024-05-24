pub mod diagnostics;
pub mod error;
pub mod notifications;
pub mod operation_stream;
pub mod requests;
use self::{error::EspxLsHandleError, operation_stream::BufferOpStreamResult};
use crate::handle::operation_stream::BufferOpStreamHandler;
use crossbeam_channel::Sender;
use diagnostics::EspxDiagnostic;
use log::{error, info, warn};
use lsp_server::{Message, Notification, RequestId, Response};
use lsp_types::{
    ApplyWorkspaceEditParams, GotoDefinitionResponse, HoverContents, ProgressParams,
    ProgressParamsValue, ProgressToken, PublishDiagnosticsParams, ShowMessageParams,
    WorkDoneProgress, WorkDoneProgressBegin, WorkDoneProgressEnd, WorkDoneProgressReport,
};
pub use notifications::handle_notification;
pub use requests::handle_request;

pub type EspxLsResult<T> = Result<T, EspxLsHandleError>;

#[derive(Debug, Clone)]
pub enum BufferOperation {
    Diagnostics(EspxDiagnostic),
    WorkDone(WorkDoneProgress),
    ShowMessage(ShowMessageParams),
    WorkspaceEdit(ApplyWorkspaceEditParams),
    GotoFile {
        id: RequestId,
        response: GotoDefinitionResponse,
    },
    HoverResponse {
        id: RequestId,
        contents: HoverContents,
    },
}

impl From<WorkDoneProgress> for BufferOperation {
    fn from(value: WorkDoneProgress) -> Self {
        Self::WorkDone(value)
    }
}

impl From<EspxDiagnostic> for BufferOperation {
    fn from(value: EspxDiagnostic) -> Self {
        Self::Diagnostics(value)
    }
}

impl From<ShowMessageParams> for BufferOperation {
    fn from(value: ShowMessageParams) -> Self {
        Self::ShowMessage(value)
    }
}

impl From<ApplyWorkspaceEditParams> for BufferOperation {
    fn from(value: ApplyWorkspaceEditParams) -> Self {
        Self::WorkspaceEdit(value)
    }
}

pub fn handle_other(msg: Message) -> EspxLsResult<BufferOpStreamHandler> {
    warn!("unhandled message {:?}", msg);
    Ok(BufferOpStreamHandler::new())
}

impl BufferOperation {
    pub async fn do_operation(
        self,
        sender: Sender<Message>,
    ) -> BufferOpStreamResult<Sender<Message>> {
        match self {
            BufferOperation::WorkDone(work) => {
                let method = match work {
                    WorkDoneProgress::Begin(_) => "window/workDoneProgress/create",
                    WorkDoneProgress::Report(_) | WorkDoneProgress::End(_) => "$/progress",
                };

                sender.send(Message::Notification(Notification {
                    method: method.to_string(),
                    params: serde_json::to_value(ProgressParams {
                        token: ProgressToken::Number(0),
                        value: ProgressParamsValue::WorkDone(work),
                    })?,
                }))?;
            }
            BufferOperation::WorkspaceEdit(edit) => {
                sender.send(Message::Notification(Notification {
                    method: "workspace/applyEdit".to_string(),
                    params: serde_json::to_value(edit)?,
                }))?;
            }
            BufferOperation::ShowMessage(message_params) => {
                sender.send(Message::Notification(Notification {
                    method: "window/showMessage".to_string(),
                    params: serde_json::to_value(message_params)?,
                }))?;
            }

            BufferOperation::GotoFile { id, response } => {
                let result = serde_json::to_value(response).ok();
                info!("SENDING GOTO FILE RESPONSE");

                sender.send(Message::Response(Response {
                    id,
                    result,
                    error: None,
                }))?;
            }

            BufferOperation::HoverResponse { contents, id } => {
                let result = match serde_json::to_value(&lsp_types::Hover {
                    contents,
                    range: None,
                }) {
                    Ok(jsn) => Some(jsn),
                    Err(err) => {
                        error!("Fail to parse hover_response: {:?}", err);
                        None
                    }
                };
                info!("SENDING HOVER RESPONSE. ID: {:?}", id);
                sender.send(Message::Response(Response {
                    id,
                    result,
                    error: None,
                }))?;
            }

            BufferOperation::Diagnostics(diag) => match diag {
                EspxDiagnostic::Publish(diag_params) => {
                    info!("PUBLISHING DIAGNOSTICS: {:?}", diag_params);
                    if let Some(params) = serde_json::to_value(diag_params).ok() {
                        sender.send(Message::Notification(Notification {
                            method: "textDocument/publishDiagnostics".to_string(),
                            params,
                        }))?;
                    }
                }

                EspxDiagnostic::ClearDiagnostics(uri) => {
                    info!("CLEARING DIAGNOSTICS");
                    let diag_params = PublishDiagnosticsParams {
                        uri,
                        diagnostics: vec![],
                        version: None,
                    };
                    if let Some(params) = serde_json::to_value(diag_params).ok() {
                        sender.send(Message::Notification(Notification {
                            method: "textDocument/publishDiagnostics".to_string(),
                            params,
                        }))?;
                    }
                }
            }, // Some(BufferOperation::CodeActionExecute(executor)) => {
               //     let cache_mut = &mut state.get_write()?.cache;
               //     sender = executor.execute(connection.sender, cache_mut)?;
               //
               //     Ok(())
               // }
               //
               // Some(BufferOperation::CodeActionRequest { response, id }) => {
               //     info!("CODE ACTION REQUEST: {:?}", response);
               //     let _ = sender.send(Message::Response(Response {
               //         id,
               //         result: serde_json::to_value(response).ok(),
               //         error: None,
               //     }))?;
               //     Ok(())
               // }
               //     None => continue,
        }
        return Ok(sender);
    }
}
