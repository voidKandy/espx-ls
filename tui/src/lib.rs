mod agents;
pub mod config;
pub mod database;
pub mod embeddings;
mod error;
pub mod handle;
pub mod interact;
pub mod state;
pub mod telemetry;
pub mod util;
use crate::handle::buffer_operations::BufferOpChannelStatus;
use anyhow::Result;
use config::Config;
use lsp_server::{Connection, Message, Notification};
use lsp_types::{
    CodeActionProviderCapability, DiagnosticServerCapabilities, InitializeParams, MessageType,
    ProgressParams, ProgressToken, ServerCapabilities, ShowMessageParams,
    TextDocumentSyncCapability, TextDocumentSyncKind, TextDocumentSyncOptions,
    TextDocumentSyncSaveOptions, WorkDoneProgress, WorkDoneProgressBegin, WorkDoneProgressEnd,
    WorkDoneProgressOptions, WorkDoneProgressReport,
};
use state::SharedState;
use tracing::{debug, info, warn};

async fn main_loop(
    mut connection: Connection,
    params: serde_json::Value,
    mut state: SharedState,
    // config: Config,
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

    let mut w = state.get_write().expect("failed to get write");
    let model_message = match &w.agents {
        Some(agents) => format!("Model Config Loaded For: {:?}", agents.config.provider),
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

    let database_message = {
        match &w.database {
            Some(db) => {
                format!(
                    "Database {}\nNamespace: {}",
                    db.config.database, db.config.namespace
                )
            }
            None => "Did not connect to a database".to_owned(),
        }
    };

    drop(w);

    connection.sender.send(Message::Notification(Notification {
        method: "$/progress".to_string(),
        params: serde_json::to_value(ProgressParams {
            token: ProgressToken::String("Initializing".to_owned()),
            value: lsp_types::ProgressParamsValue::WorkDone(WorkDoneProgress::End(
                WorkDoneProgressEnd {
                    // message: None,
                    message: Some(database_message),
                },
            )),
        })?,
    }))?;

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
                            // connection.sender = buffer_op.do_operation(connection.sender).await?;
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
    let config = Config::init();
    let state = SharedState::init(config).await?;
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
