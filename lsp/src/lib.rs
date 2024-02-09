mod espx_env;
mod handle;
mod htmx;
mod parsing;
mod text_store;
// mod tree_sitter;
// mod tree_sitter_querier;

use anyhow::Result;
use espionox::environment::agent::language_models::openai::gpt::streaming_utils::CompletionStreamStatus;
use htmx::EspxCompletion;
use log::{debug, error, info, warn};
use lsp_types::{
    CodeActionProviderCapability, CompletionItem, CompletionItemKind, CompletionList,
    HoverContents, InitializeParams, LanguageString, MarkedString, MessageType, OneOf,
    ServerCapabilities, ShowMessageParams, ShowMessageRequestParams, TextDocumentSyncCapability,
    TextDocumentSyncKind, TextDocumentSyncOptions, TextDocumentSyncSaveOptions,
    WorkDoneProgressOptions,
};

use lsp_server::{Connection, Message, Notification, Request, Response};
use uuid::Uuid;

use crate::{
    espx_env::{
        init_static_env_and_handle, stream_prompt_main_agent, ENVIRONMENT, MAIN_AGENT_HANDLE,
    },
    handle::{handle_notification, handle_other, handle_request, EspxResult},
    htmx::init_hx_tags,
    text_store::init_text_store,
};

fn to_completion_list(items: Vec<EspxCompletion>) -> CompletionList {
    return CompletionList {
        is_incomplete: true,
        items: items
            .iter()
            .map(|x| CompletionItem {
                label: x.name.clone(),
                label_details: None,
                kind: Some(CompletionItemKind::TEXT),
                detail: Some(x.desc.clone()),
                documentation: None,
                deprecated: Some(false),
                preselect: None,
                sort_text: None,
                filter_text: None,
                insert_text: None,
                insert_text_format: None,
                insert_text_mode: None,
                text_edit: x.edit.clone(),
                additional_text_edits: None,
                command: None,
                commit_characters: None,
                data: None,
                tags: None,
            })
            .collect(),
    };
}

async fn main_loop(connection: Connection, params: serde_json::Value) -> Result<()> {
    let _params: InitializeParams = serde_json::from_value(params).unwrap();

    info!("STARTING EXAMPLE MAIN LOOP");

    for msg in &connection.receiver {
        error!("connection received message: {:?}", msg);
        let result = match msg {
            Message::Notification(not) => handle_notification(not),
            Message::Request(req) => handle_request(req).await,
            _ => handle_other(msg),
        };

        match match result {
            Some(EspxResult::AttributeCompletion(c)) => {
                let str = match serde_json::to_value(&to_completion_list(c.items)) {
                    Ok(s) => s,
                    Err(_) => continue,
                };

                // TODO: block requests that have been cancelled
                connection.sender.send(Message::Response(Response {
                    id: c.id,
                    result: Some(str),
                    error: None,
                }))
            }

            Some(EspxResult::ShowMessage(message)) => {
                connection.sender.send(Message::Notification(Notification {
                    method: "window/showMessage".to_string(),
                    params: serde_json::to_value(ShowMessageRequestParams {
                        typ: MessageType::INFO,
                        message,
                        actions: None,
                    })?,
                }))
            }

            Some(EspxResult::CodeAction { action, id }) => {
                connection.sender.send(Message::Response(Response {
                    id,
                    result: serde_json::to_value(action).ok(),
                    error: None,
                }))
            }

            Some(EspxResult::PromptHover(mut hover_res)) => {
                debug!("main_loop - hover response: {:?}", hover_res);
                if hover_res.handler.is_none() {
                    let handler = stream_prompt_main_agent(&hover_res.value).await.unwrap();
                    hover_res.handler = Some(handler);

                    let hover_response = lsp_types::Hover {
                        contents: HoverContents::Scalar(MarkedString::LanguageString(
                            LanguageString {
                                language: "html".to_string(),
                                value: "Processing prompt...".to_string(),
                            },
                        )),
                        range: None,
                    };

                    let json = serde_json::to_value(&hover_response)
                        .expect("Failed hover response to JSON");

                    connection
                        .sender
                        .send(Message::Response(Response {
                            id: hover_res.id.clone(),
                            result: Some(json),
                            error: None,
                        }))
                        .unwrap();
                }

                let handler = hover_res.handler.unwrap();
                let mut handler = handler.lock().await;
                let environment = ENVIRONMENT
                    .get()
                    .expect("can't get static env")
                    .lock()
                    .unwrap();

                let h = MAIN_AGENT_HANDLE
                    .get()
                    .expect("Can't get static agent")
                    .lock()
                    .expect("Can't lock static agent");
                while let Some(CompletionStreamStatus::Working(token)) =
                    handler.receive(&h.id, environment.clone_sender()).await
                {
                    let hover_response = lsp_types::Hover {
                        contents: HoverContents::Scalar(MarkedString::LanguageString(
                            LanguageString {
                                language: "html".to_string(),
                                value: token.to_string(),
                            },
                        )),
                        range: None,
                    };
                    let json = serde_json::to_value(&hover_response)
                        .expect("Failed hover response to JSON");

                    connection
                        .sender
                        .send(Message::Response(Response {
                            id: Uuid::new_v4().to_string().into(),
                            result: Some(json),
                            error: None,
                        }))
                        .unwrap();
                }
                Ok(())
            }
            Some(EspxResult::DocumentEdit(edit)) => {
                debug!("main_loop - docedit response: {:?}", edit);
                let textedit = lsp_types::TextDocumentEdit {
                    text_document: {
                        lsp_types::OptionalVersionedTextDocumentIdentifier {
                            uri: edit.uri,
                            version: None,
                        }
                    },
                    edits: vec![OneOf::Left(lsp_types::TextEdit {
                        range: edit.range,
                        new_text: edit.new_text,
                    })],
                };
                let str = match serde_json::to_value(&textedit) {
                    Ok(s) => s,
                    Err(err) => {
                        error!("Fail to parse edit_response: {:?}", err);
                        return Err(anyhow::anyhow!("Fail to parse edit_response"));
                    }
                };
                debug!("Connection sent text edit: {:?}", str);
                connection.sender.send(Message::Response(Response {
                    id: edit.id,
                    result: Some(str),
                    error: None,
                }))
            }
            None => continue,
        } {
            Ok(_) => {}
            Err(e) => error!("failed to send response: {:?}", e),
        };
    }

    Ok(())
}

#[tokio::main]
pub async fn start_lsp() -> Result<()> {
    init_static_env_and_handle().await;
    init_text_store();
    init_text_store();
    init_hx_tags();

    // Note that  we must have our logging only write out to stderr.
    info!("starting generic LSP server");

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

#[cfg(test)]
mod test {
    use anyhow::Result;

    #[test]
    fn test_byte_col() -> Result<()> {
        /*
                let source = "oeunth";

                let (line, col) = byte_pos_to_line_col(source.as_str(), msg.position.0);
                assert_eq!(line, 9);
                assert_eq!(col, 9);

                let (line, col) = byte_pos_to_line_col(source.as_str(), msg.position.1);
                assert_eq!(line, 9);
                assert_eq!(col, 21);
        /005   test/
        /005   test/

                */
        Ok(())
    }
}
