mod cache;
mod config;
mod database;
mod error;
mod espx_env;
mod handle;

use anyhow::Result;
use handle::runes::ActionRune;
use log::{error, info, warn};
use lsp_types::{
    CodeActionProviderCapability, DiagnosticServerCapabilities, InitializeParams, MessageType,
    PublishDiagnosticsParams, ServerCapabilities, ShowMessageRequestParams,
    TextDocumentSyncCapability, TextDocumentSyncKind, TextDocumentSyncOptions,
    TextDocumentSyncSaveOptions, WorkDoneProgressOptions,
};

use lsp_server::{Connection, Message, Notification, Response};

use crate::{
    database::DB,
    espx_env::init_espx_env,
    handle::diagnostics::EspxDiagnostic,
    handle::{handle_notification, handle_other, handle_request, EspxResult},
};

async fn main_loop(mut connection: Connection, params: serde_json::Value) -> Result<()> {
    let _params: InitializeParams = serde_json::from_value(params).unwrap();

    connection.sender.send(Message::Notification(Notification {
        method: "window/showMessage".to_string(),
        params: serde_json::to_value(ShowMessageRequestParams {
            typ: MessageType::INFO,
            message: String::from("Espx LS Running"),
            actions: None,
        })?,
    }))?;

    for msg in &connection.receiver {
        error!("connection received message: {:?}", msg);
        let result = match msg {
            Message::Notification(not) => handle_notification(not).await,
            Message::Request(req) => handle_request(req).await,
            _ => handle_other(msg),
        };

        match match result {
            Some(EspxResult::Diagnostics(diag)) => {
                match diag {
                    EspxDiagnostic::Publish(diags) => {
                        info!("PUBLISHING DIAGNOSTICS");
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

            Some(EspxResult::CodeActionExecute(executor)) => {
                let url = executor.url().clone();
                let (sender, burn) = executor.execute(connection.sender)?;

                connection.sender = burn.burn_into_cache(url, sender)?;
                Ok(())
            }

            Some(EspxResult::CodeActionRequest { response, id }) => {
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
    init_espx_env().await;
    // Namespace should likely be name of outermost directory
    DB.read().unwrap().connect_db("Main", "Main").await;
    // init_store();
    // config::echo_markerfile();

    // Note that  we must have our logging only write out to stderr.
    info!("starting LSP server");

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

        ..Default::default()
    })
    .unwrap();

    let initialization_params = connection.initialize(server_capabilities)?;
    main_loop(connection, initialization_params).await?;
    io_threads.join()?;
    // Shut down gracefully.
    warn!("shutting down server");
    Ok(())
}
