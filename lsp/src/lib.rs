mod config;
pub mod embeddings;
mod error;
mod handle;
mod parsing;
mod state;
mod tests;
use anyhow::Result;
use config::GLOBAL_CONFIG;
use lsp_server::{Connection, Message, Notification};
use lsp_types::{
    CodeActionProviderCapability, DiagnosticServerCapabilities, InitializeParams, ProgressParams,
    ProgressToken, ServerCapabilities, TextDocumentSyncCapability, TextDocumentSyncKind,
    TextDocumentSyncOptions, TextDocumentSyncSaveOptions, WorkDoneProgress, WorkDoneProgressBegin,
    WorkDoneProgressEnd, WorkDoneProgressOptions, WorkDoneProgressReport,
};
use state::SharedGlobalState;
use tracing::{debug, info, warn};

use crate::handle::buffer_operations::BufferOpChannelStatus;

async fn main_loop(
    mut connection: Connection,
    params: serde_json::Value,
    state: SharedGlobalState,
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
    let db_message = match &GLOBAL_CONFIG.database {
        Some(dconf) => format!(
            "Database {} running on {}:{}\nNamespace: {}",
            dconf.database,
            dconf.host.as_ref().unwrap_or(&"0.0.0.0".to_owned()),
            dconf.port,
            dconf.namespace
        ),
        None => "No Database info in your config file, persistence unavailable.".to_owned(),
    };

    connection.sender.send(Message::Notification(Notification {
        method: "$/progress".to_string(),
        params: serde_json::to_value(ProgressParams {
            token: ProgressToken::String("Initializing".to_owned()),
            value: lsp_types::ProgressParamsValue::WorkDone(WorkDoneProgress::End(
                WorkDoneProgressEnd {
                    message: Some(db_message),
                },
            )),
        })?,
    }))?;

    for msg in &connection.receiver {
        let mut buffer_op_stream_handler = match msg {
            Message::Notification(not) => {
                handle::notifications::handle_notification(not, state.clone()).await?
            }
            Message::Request(req) => handle::requests::handle_request(req, state.clone()).await?,
            _ => handle::handle_other(msg)?,
        };

        while let Some(status) = buffer_op_stream_handler.receiver.recv().await {
            match status? {
                BufferOpChannelStatus::Finished => break,
                BufferOpChannelStatus::Working(buffer_op) => {
                    connection.sender = buffer_op.do_operation(connection.sender).await?;
                }
            }
        }
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
