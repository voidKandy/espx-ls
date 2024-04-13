mod burns;
mod cache;
mod config;
mod database;
mod error;
mod espx_env;
mod handle;
mod state;

use anyhow::Result;
use log::{error, info, warn};
use lsp_types::{
    CodeActionProviderCapability, DiagnosticServerCapabilities, GotoDefinitionResponse,
    InitializeParams, MessageType, PublishDiagnosticsParams, ServerCapabilities,
    ShowMessageRequestParams, TextDocumentSyncCapability, TextDocumentSyncKind,
    TextDocumentSyncOptions, TextDocumentSyncSaveOptions, WorkDoneProgressOptions,
};

use lsp_server::{Connection, Message, Notification, Response};
use state::SharedGlobalState;

use crate::{
    database::DB,
    espx_env::init_espx_env,
    handle::{
        diagnostics::EspxDiagnostic, handle_notification, handle_other, handle_request,
        BufferOperation,
    },
};

async fn main_loop(
    mut connection: Connection,
    params: serde_json::Value,
    mut state: SharedGlobalState,
) -> Result<()> {
    let _params: InitializeParams = serde_json::from_value(params).unwrap();

    connection.sender.send(Message::Notification(Notification {
        method: "window/showMessage".to_string(),
        params: serde_json::to_value(ShowMessageRequestParams {
            typ: MessageType::INFO,
            message: String::from("🕵 Espx LS Running 🕵"),
            actions: None,
        })?,
    }))?;

    for msg in &connection.receiver {
        error!("connection received message: {:?}", msg);
        let result = match msg {
            Message::Notification(not) => handle_notification(not, state.clone()).await,
            Message::Request(req) => handle_request(req, state.clone()).await,
            _ => handle_other(msg),
        };

        match match result? {
            Some(BufferOperation::GotoFile { id, response }) => {
                let result = serde_json::to_value(response).ok();
                info!("SENDING GOTO FILE RESPONSE");

                connection.sender.send(Message::Response(Response {
                    id,
                    result,
                    error: None,
                }))?;
                Ok(())
            }
            Some(BufferOperation::HoverResponse { contents, id }) => {
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
                connection.sender.send(Message::Response(Response {
                    id,
                    result,
                    error: None,
                }))?;
                Ok(())
            }
            Some(BufferOperation::Diagnostics(diag)) => {
                match diag {
                    EspxDiagnostic::Publish(diags) => {
                        info!("PUBLISHING DIAGNOSTICS: {:?}", diags);
                        for diag_params in diags.into_iter() {
                            if let Some(params) = serde_json::to_value(diag_params).ok() {
                                connection.sender.send(Message::Notification(Notification {
                                    method: "textDocument/publishDiagnostics".to_string(),
                                    params,
                                }))?;
                            }
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
                            connection.sender.send(Message::Notification(Notification {
                                method: "textDocument/publishDiagnostics".to_string(),
                                params,
                            }))?;
                        }
                    }
                }
                Ok::<(), anyhow::Error>(())
            }

            Some(BufferOperation::CodeActionExecute(executor)) => {
                let cache_mut = &mut state.get_write()?.cache;
                connection.sender = executor.execute(connection.sender, cache_mut)?;

                Ok(())
            }

            Some(BufferOperation::CodeActionRequest { response, id }) => {
                info!("CODE ACTION REQUEST: {:?}", response);
                let _ = connection.sender.send(Message::Response(Response {
                    id,
                    result: serde_json::to_value(response).ok(),
                    error: None,
                }))?;
                Ok(())
            }

            None => continue,
        } {
            Ok(_) => {}
            Err(e) => error!("failed to send response: {:?}", e),
        };
    }

    DB.write().unwrap().kill_handle().await?;
    Ok(())
}

#[tokio::main]
pub async fn start_lsp() -> Result<()> {
    info!("starting LSP server");

    let state = SharedGlobalState::default();
    init_espx_env(&state).await;

    info!("Espionox Environment initialized");
    // Namespace should likely be name of outermost directory
    DB.read().unwrap().connect_db("Main", "Main").await;

    // Create the transport. Includes the stdio (stdin and stdout) versions but this could
    // also be implemented to use sockets or HTTP.
    let (connection, io_threads) = Connection::stdio();

    let text_document_sync = Some(TextDocumentSyncCapability::Options(
        TextDocumentSyncOptions {
            open_close: Some(true),
            save: Some(TextDocumentSyncSaveOptions::SaveOptions(
                lsp_types::SaveOptions {
                    include_text: Some(true),
                },
            )),
            change: Some(TextDocumentSyncKind::FULL),

            ..Default::default()
        },
    ));
    let server_capabilities = serde_json::to_value(ServerCapabilities {
        text_document_sync,
        completion_provider: Some(lsp_types::CompletionOptions {
            resolve_provider: Some(false),
            trigger_characters: Some(vec!["?".to_string(), "\"".to_string(), " ".to_string()]),
            work_done_progress_options: WorkDoneProgressOptions {
                work_done_progress: None,
            },
            all_commit_characters: None,
            completion_item: None,
        }),
        code_action_provider: Some(CodeActionProviderCapability::Simple(true)),
        hover_provider: Some(lsp_types::HoverProviderCapability::Simple(true)),
        diagnostic_provider: Some(DiagnosticServerCapabilities::Options(
            lsp_types::DiagnosticOptions::default(),
        )),
        definition_provider: Some(lsp_types::OneOf::Left(true)),
        ..Default::default()
    })
    .unwrap();

    let initialization_params = connection.initialize(server_capabilities)?;
    main_loop(connection, initialization_params, state).await?;
    io_threads.join()?;
    // Shut down gracefully.
    warn!("shutting down server");
    Ok(())
}
