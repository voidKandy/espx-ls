use super::config::test_config;
use espx_lsp_server::{
    handle::buffer_operations::BufferOpChannelHandler,
    state::{LspState, SharedState},
};
use lsp_server::RequestId;
use lsp_types::{
    GotoDefinitionParams, HoverParams, PartialResultParams, Position, TextDocumentIdentifier,
    TextDocumentPositionParams, Uri, WorkDoneProgressParams,
};
use serde::Serialize;
use std::sync::LazyLock;
use tracing::{info, subscriber::set_global_default, Subscriber};
use tracing_bunyan_formatter::{BunyanFormattingLayer, JsonStorageLayer};
use tracing_log::LogTracer;
use tracing_subscriber::{fmt::MakeWriter, layer::SubscriberExt, EnvFilter, Registry};

pub static TEST_TRACING: LazyLock<()> = LazyLock::new(|| {
    let default_filter_level = "info".to_string();
    let subscriber_name = "lsp".to_string();

    let sub = get_subscriber(subscriber_name, default_filter_level, std::io::stdout);
    init_subscriber(sub);
    info!("test tracing initialized");
});

pub async fn test_state(database: bool) -> SharedState {
    SharedState::init(test_config(database).unwrap())
        .await
        .unwrap()
}

pub fn test_buff_op_channel() -> BufferOpChannelHandler {
    BufferOpChannelHandler::new()
}

pub fn into_lsp_notification<P: Serialize>(params: P, method: &str) -> lsp_server::Notification {
    let params = serde_json::to_value(params).expect("could not serialize");
    lsp_server::Notification {
        method: method.to_string(),
        params,
    }
}

pub fn into_lsp_request<P: Serialize>(
    params: P,
    id: impl Into<RequestId>,
    method: &str,
) -> lsp_server::Request {
    let params = serde_json::to_value(params).expect("could not serialize");
    lsp_server::Request {
        id: id.into(),
        method: method.to_string(),
        params,
    }
}

pub fn create_gotodef_params(position: Position, doc_uri: Uri) -> GotoDefinitionParams {
    let partial_result_params = PartialResultParams {
        partial_result_token: None,
    };

    let work_done_progress_params = WorkDoneProgressParams {
        work_done_token: None,
    };

    let text_document_position_params = TextDocumentPositionParams {
        text_document: TextDocumentIdentifier { uri: doc_uri },
        position,
    };

    GotoDefinitionParams {
        text_document_position_params,
        work_done_progress_params,
        partial_result_params,
    }
}

pub fn create_hover_params(position: Position, doc_uri: Uri) -> HoverParams {
    let work_done_progress_params = WorkDoneProgressParams {
        work_done_token: None,
    };

    let text_document_position_params = TextDocumentPositionParams {
        text_document: TextDocumentIdentifier { uri: doc_uri },
        position,
    };

    HoverParams {
        text_document_position_params,
        work_done_progress_params,
    }
}

fn get_subscriber<Sink>(
    name: String,
    env_filter: String,
    sink: Sink,
) -> impl Subscriber + Send + Sync
where
    Sink: for<'a> MakeWriter<'a> + Send + Sync + 'static,
{
    let env_filter =
        EnvFilter::try_from_default_env().unwrap_or_else(|_| EnvFilter::new(env_filter));
    let formatting_layer = BunyanFormattingLayer::new(name, sink);

    Registry::default()
        .with(env_filter)
        .with(JsonStorageLayer)
        .with(formatting_layer)
}

fn init_subscriber(subscriber: impl Subscriber + Send + Sync) {
    LogTracer::init().expect("Failed to set logger");
    set_global_default(subscriber).expect("Failed to set subscriber.");
}
