mod burns;
mod cache;
mod parsing;

mod config;
mod database;
mod error;
mod espx_env;
mod handle;
mod state;

use anyhow::Result;
use log::{error, info, warn};
use lsp_types::{
    CodeActionProviderCapability, DiagnosticServerCapabilities, InitializeParams, MessageType,
    ServerCapabilities, ShowMessageRequestParams, TextDocumentSyncCapability, TextDocumentSyncKind,
    TextDocumentSyncOptions, TextDocumentSyncSaveOptions, WorkDoneProgressOptions,
};

use lsp_server::{Connection, Message, Notification};
use state::SharedGlobalState;

use crate::{
    database::DB,
    handle::{
        handle_notification, handle_other, handle_request, operation_stream::BufferOpStreamStatus,
    },
};

async fn main_loop(
    mut connection: Connection,
    params: serde_json::Value,
    state: SharedGlobalState,
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
        let mut buffer_op_stream_handler = match msg {
            Message::Notification(not) => handle_notification(not, state.clone()).await?,
            Message::Request(req) => handle_request(req, state.clone()).await?,
            _ => handle_other(msg)?,
        };

        while let Some(status) = buffer_op_stream_handler.receiver.recv().await {
            match status? {
                BufferOpStreamStatus::Finished => break,
                BufferOpStreamStatus::Working(buffer_op) => {
                    connection.sender = buffer_op.do_operation(connection.sender).await?;
                }
            }
        }
        // while let Ok(BufferOpStreamStatus::Working(buffer_op)) = buffer_op_stream_handler
        //     .receiver
        //     .recv()
        //     .await
        //     .ok_or(BufferOpStreamError::Undefined(anyhow!(
        //         "Some error occurred while receiving"
        //     )))?
        // {}
    }

    DB.write().unwrap().kill_handle().await?;
    Ok(())
}

#[tokio::main]
pub async fn start_lsp() -> Result<()> {
    info!("starting LSP server");

    let state = SharedGlobalState::init().await;
    info!("State initialized");
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
