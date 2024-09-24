mod config;
pub mod commands;
pub mod embeddings;
mod error;
mod handle;
mod parsing;
mod state;
#[cfg(test)]
mod tests;
pub mod util;
use anyhow::Result;
use config::GLOBAL_CONFIG;
use lsp_server::{Connection, Message, Notification};
use lsp_types::{
    CodeActionProviderCapability, DiagnosticServerCapabilities, InitializeParams, MessageType,
    ProgressParams, ProgressToken, ServerCapabilities, ShowMessageParams,
    TextDocumentSyncCapability, TextDocumentSyncKind, TextDocumentSyncOptions,
    TextDocumentSyncSaveOptions, WorkDoneProgress, WorkDoneProgressBegin, WorkDoneProgressEnd,
    WorkDoneProgressOptions, WorkDoneProgressReport,
};
use state::{store::error::StoreError, SharedGlobalState};
use tracing::{debug, info, warn};

use crate::handle::buffer_operations::BufferOpChannelStatus;

async fn main_loop(
    mut connection: Connection,
    params: serde_json::Value,
    mut state: SharedGlobalState,
) -> Result<()> {
    let _params: InitializeParams = serde_json::from_value(params).unwrap();

    connection.sender.send(Message::Notification(Notification {
        method: "window/workDoneProgress/create".to_string(),
        params: serde_json::to_value(ProgressParams {
            token: ProgressToken::String("Initializing".to_owned()),
            value: lsp_types::ProgressParamsValue::WorkDone(WorkDoneProgress::Begin(
                WorkDoneProgressBegin {
                    title: "Initializing".to_owned(),
                    ..Default::default()
                },
            )),
        })?,
    }))?;

    let model_message = match &GLOBAL_CONFIG.model {
        Some(mconf) => format!("Model Config Loaded For: {:?}", mconf.provider),
        None => "No model in your config file, AI will be unusable.".to_owned(),
    };

    connection.sender.send(Message::Notification(Notification {
        method: "$/progress".to_string(),
        params: serde_json::to_value(ProgressParams {
            token: ProgressToken::String("Initializing".to_owned()),
            value: lsp_types::ProgressParamsValue::WorkDone(WorkDoneProgress::Report(
                WorkDoneProgressReport {
                    message: Some(model_message),
                    percentage: Some(50),
                    ..Default::default()
                },
            )),
        })?,
    }))?;

    let mut w = state.get_write().expect("failed to get write");
    let database_message = {
        if w.database.is_some() {
            w.try_update_from_database().await?;
            let dconf = GLOBAL_CONFIG.database.as_ref().unwrap();
            format!(
                "Database {}\nNamespace: {}",
                dconf.database, dconf.namespace
            )
        } else {
            "Did not connect to a database".to_owned()
        }
    };

    drop(w);

    connection.sender.send(Message::Notification(Notification {
        method: "$/progress".to_string(),
        params: serde_json::to_value(ProgressParams {
            token: ProgressToken::String("Initializing".to_owned()),
            value: lsp_types::ProgressParamsValue::WorkDone(WorkDoneProgress::End(
                WorkDoneProgressEnd {
                    message: Some(database_message),
                },
            )),
        })?,
    }))?;

    // if let Err(err) = state.get_write()?.store.try_update_from_database().await {
    //     if let StoreError::NotPresent(_) = err {
    //     } else {
    //         return Err(err.into());
    //     }
    // }

    for msg in &connection.receiver {
        match match msg {
            Message::Notification(not) => {
                handle::notifications::handle_notification(not, state.clone()).await
            }
            Message::Request(req) => handle::requests::handle_request(req, state.clone()).await,
            _ => handle::handle_other(msg),
        } {
            Ok(mut buffer_op_channel_handler) => {
                while let Some(status) = buffer_op_channel_handler.receiver.recv().await {
                    match status? {
                        BufferOpChannelStatus::Finished => break,
                        BufferOpChannelStatus::Working(buffer_op) => {
                            connection.sender = buffer_op.do_operation(connection.sender).await?;
                        }
                    }
                }
            }
            Err(err) => {
                warn!("error in handler: {}", err);
                connection.sender.send(Message::Notification(Notification {
                    method: "window/showMessage".to_string(),
                    params: serde_json::to_value(ShowMessageParams {
                        typ: MessageType::ERROR,
                        message: format!("Handler encounted an error: {}", err),
                    })?,
                }))?;
            }
        }
        debug!("finished processing message, moving on");
    }

    Ok(())
}

#[tokio::main]
pub async fn start_lsp() -> Result<()> {
    info!("starting LSP server");
    let state = SharedGlobalState::init().await?;
    info!("State initialized");

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
            change: Some(TextDocumentSyncKind::INCREMENTAL),

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
    Ok(())
}
