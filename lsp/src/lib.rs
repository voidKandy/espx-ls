pub mod burns;
pub mod config;
pub mod embeddings;
pub mod error;
pub mod espx_env;
pub mod handle;
pub mod parsing;
pub mod state;
pub mod store;
pub mod util;
use crate::handle::{
    handle_notification, handle_other, handle_request, operation_stream::BufferOpStreamStatus,
};
use anyhow::Result;
use config::GLOBAL_CONFIG;
use log::{error, info, warn};
use lsp_server::{Connection, Message, Notification};
use lsp_types::{
    CodeActionProviderCapability, DiagnosticServerCapabilities, InitializeParams, MessageType,
    ServerCapabilities, ShowMessageRequestParams, TextDocumentSyncCapability, TextDocumentSyncKind,
    TextDocumentSyncOptions, TextDocumentSyncSaveOptions, WorkDoneProgressOptions,
};
use state::SharedGlobalState;

async fn main_loop(
    mut connection: Connection,
    params: serde_json::Value,
    mut state: SharedGlobalState,
) -> Result<()> {
    let _params: InitializeParams = serde_json::from_value(params).unwrap();
    // state.get_write()?.store.update_from_root().await?;

    let model_message = match &GLOBAL_CONFIG.model {
        Some(mconf) => format!("Model Config Loaded For: {:?}", mconf.provider),
        None => "No model in your config file, AI will be unusable.".to_owned(),
    };

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

    // THIS SHOULD BE REPLACED BY $/progress
    connection.sender.send(Message::Notification(Notification {
        method: "window/showMessage".to_string(),
        params: serde_json::to_value(ShowMessageRequestParams {
            typ: MessageType::INFO,
            message: format!("{}\n{}", model_message, db_message),
            actions: None,
        })?,
    }))?;

    // POPULATE DOC STORE

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
    }

    if let Some(mut db) = state.get_write()?.store.db.take() {
        db.client.kill_handle().await?;
    }
    Ok(())
}

#[tokio::main]
pub async fn start_lsp() -> Result<()> {
    info!("starting LSP server");
    let state = SharedGlobalState::init().await?;
    info!("State initialized");
    // Namespace should likely be name of outermost directory

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
