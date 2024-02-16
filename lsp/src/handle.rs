use std::collections::HashMap;

use crate::{
    espx_env::{io_prompt_agent, stream_prompt_agent, WATCHER_AGENT_HANDLE},
    htmx::{espx_completion, espx_hover, espx_text_edit, EspxCompletion},
    parsing::{self, get_prompt_and_position, parse_for_prompt, Position as ParsedPosition},
    text_store::{
        get_text_document, get_text_document_current, update_doc_store, Document, DOCUMENT_STORE,
    },
};
use espionox::environment::{
    agent::{
        language_models::openai::gpt::streaming_utils::StreamedCompletionHandler, memory::ToMessage,
    },
    dispatch::ThreadSafeStreamCompletionHandler,
};
use log::{debug, error, warn};
use lsp_server::{Message, Notification, Request, RequestId};
use lsp_types::{
    CodeAction, CodeActionOrCommand, CodeActionParams, CodeActionResponse, CodeLens,
    CodeLensParams, Command, CompletionContext, CompletionParams, CompletionTriggerKind,
    DidChangeTextDocumentParams, ExecuteCommandParams, HoverParams, OneOf,
    OptionalVersionedTextDocumentIdentifier, Position, Range, TextDocumentEdit, TextDocumentItem,
    TextEdit, Url,
};
use serde_json::json;

#[derive(serde::Deserialize, Debug)]
struct TextDocumentOpen {
    #[serde(rename = "textDocument")]
    text_document: TextDocumentItem,
}

#[derive(Debug)]
pub struct HtmxAttributeCompletion {
    pub items: Vec<EspxCompletion>,
    pub id: RequestId,
}

#[derive(Debug)]
pub struct EspxHoverResult {
    pub id: RequestId,
    pub value: String,
    pub handler: Option<ThreadSafeStreamCompletionHandler>,
}

pub enum EspxCommand {
    PromptCodebaseModel {},
}

#[derive(Debug)]
pub enum EspxResult {
    // Unused!!!
    AttributeCompletion(HtmxAttributeCompletion),
    PromptHover(EspxHoverResult),
    DocumentEdit(super::htmx::EspxDocumentEdit),

    ShowMessage(String),
    CodeAction {
        action: CodeActionResponse,
        id: RequestId,
    },
}

#[allow(non_snake_case)]
fn handle_didChange(noti: Notification) -> Option<EspxResult> {
    let text_document_changes: DidChangeTextDocumentParams =
        serde_json::from_value(noti.params).ok()?;

    debug!("didChange Handle CHANGES: {:?}", text_document_changes);
    if text_document_changes.content_changes.len() > 1 {
        debug!("BEWARE MULTIPLE CHANGES PASSED IN THIS NOTIFICATION");
    }
    let uri = text_document_changes.text_document.uri;
    update_doc_store(&uri, &text_document_changes.content_changes[0]).ok()?;
    // for change in text_document_changes.content_changes.iter() {
    //     let _ = update_changes_record(&uri, change).ok()?;
    //     debug!("Changes record added change: {:?}", change);
    // }

    None
}

// #[allow(non_snake_case)]
// fn handle_didSave(noti: Notification) -> Option<EspxResult> {
//     let text_document_changes = match serde_json::from_value::<TextDocumentOpen>(noti.params) {
//         Ok(p) => p.text_document,
//         Err(err) => {
//             error!("handle_didSave parsing params error : {:?}", err);
//             return None;
//         }
//     };
//
//     DOCUMENT_STORE
//         .get()
//         .expect("text store not initialized")
//         .lock()
//         .expect("text store mutex poisoned")
//         .texts
//         .insert(
//             text_document_changes.uri,
//             text_document_changes.text.to_string(),
//         );
//
//     None
// }

#[allow(non_snake_case)]
fn handle_didOpen(noti: Notification) -> Option<EspxResult> {
    let text_doc_item = serde_json::from_value::<TextDocumentOpen>(noti.params).ok()?;

    DOCUMENT_STORE
        .get()
        .expect("text store not initialized")
        .lock()
        .expect("text store mutex poisoned")
        .insert(
            text_doc_item.text_document.uri,
            Document::from(text_doc_item.text_document.text.to_string()),
        );
    debug!("didOpen Handle updated DOCUMENT_STORE");

    None
}

// #[allow(non_snake_case)]
// async fn handle_completion(req: Request) -> Option<EspxResult> {
//     let completion: CompletionParams = serde_json::from_value(req.params).ok()?;
//
//     error!("handle_completion: {:?}", completion);
//
//     match completion.context {
//         Some(CompletionContext {
//             trigger_kind: CompletionTriggerKind::TRIGGER_CHARACTER,
//             ..
//         })
//         | Some(CompletionContext {
//             trigger_kind: CompletionTriggerKind::INVOKED,
//             ..
//         }) => {
//             let items = match espx_completion(completion.text_document_position).await {
//                 Some(items) => items,
//                 None => {
//                     error!("EMPTY RESULTS OF COMPLETION");
//                     return None;
//                 }
//             };
//
//             error!(
//                 "handled result: {:?}: completion result: {:?}",
//                 completion.context, items
//             );
//
//             Some(EspxResult::AttributeCompletion(HtmxAttributeCompletion {
//                 items,
//                 id: req.id,
//             }))
//         }
//         _ => {
//             error!("unhandled completion context: {:?}", completion.context);
//             None
//         }
//     }
// }

async fn handle_code_action(req: Request) -> Option<EspxResult> {
    let params: CodeActionParams = serde_json::from_value(req.params).ok()?;
    debug!("CODE ACTION REQUEST: {:?}", params);
    let mut command = None;
    let mut url = None;
    let doc = params.text_document;
    let uri = doc.uri;
    debug!("URI PARSED FROM ACTION REQUEST: {:?}", uri);

    if let Some(text) = get_text_document_current(&uri) {
        if let Some((prompt, pos)) = get_prompt_and_position(&text) {
            command = Some(Command {
                title: "Ask Question".to_string(),
                command: "prompt".to_string(),
                arguments: Some(vec![json!({"position": pos, "prompt": prompt})]),
            });
            debug!("GOT COMMAND: {:?}", command);
        }
    }
    url = Some(uri);

    let prompt_action = CodeAction {
        title: String::from("PromptTest"),
        command,
        ..Default::default()
    };

    let look_at_me_action = CodeAction {
        title: String::from("LookAtMe"),
        command: Some(Command {
            title: "Look at me".to_owned(),
            command: "look_at_me".to_owned(),
            arguments: Some(vec![json!({"uri": url.unwrap().to_string()})]),
        }),
        ..Default::default()
    };

    debug!("SHOULD RETURN SOME ACTIONS");

    Some(EspxResult::CodeAction {
        action: vec![
            CodeActionOrCommand::CodeAction(prompt_action),
            CodeActionOrCommand::CodeAction(look_at_me_action),
        ],
        id: req.id,
    })
}

async fn handle_execute_command(req: Request) -> Option<EspxResult> {
    let params = serde_json::from_value::<ExecuteCommandParams>(req.params).ok()?;
    debug!("COMMAND EXECUTION: {:?}", params);
    match params.command.as_str() {
        "prompt" => {
            if let Some(prompt) = params
                .arguments
                .iter()
                .find_map(|arg| arg.as_object()?.get("prompt")?.as_str())
            {
                debug!("USER PROMPT: {}", prompt);
                return Some(EspxResult::ShowMessage(prompt.to_owned()));
            }
        }
        "look_at_me" => {
            if let Some(url) = params
                .arguments
                .iter()
                .find_map(|arg| arg.as_object()?.get("uri")?.as_str())
            {
                debug!("LOOKING AT USER in URL: {}", url);
                let uri = Url::parse(url).ok()?;
                let doc = get_text_document(&uri)?;
                let message =
                    doc.to_message(espionox::environment::agent::memory::MessageRole::System);
                // io_prompt_agent should take a Message NOT A &str
                let response =
                    io_prompt_agent(&message.content, crate::espx_env::CopilotAgent::Watcher)
                        .await
                        .unwrap();
                return Some(EspxResult::ShowMessage(response));
            }
        }
        _ => {
            debug!("Somehow an unhandled command was executed")
        }
    }
    None
}

async fn handle_hover(req: Request) -> Option<EspxResult> {
    let completion: CompletionParams = serde_json::from_value(req.params).ok()?;
    debug!("handle_hover: {:?}", completion.context);

    let text_params = completion.text_document_position;
    debug!("handle_hover text_params: {:?}", text_params);

    let parsed_pos = parsing::get_position_from_lsp_completion(&text_params)?;
    if let ParsedPosition::UserPrompt(prompt) = parsed_pos {
        return Some(EspxResult::PromptHover(EspxHoverResult {
            id: req.id,
            value: prompt,
            handler: None,
        }));
    }
    None
}

pub async fn handle_request(req: Request) -> Option<EspxResult> {
    error!("handle_request");
    match req.method.as_str() {
        "workspace/executeCommand" => handle_execute_command(req).await,
        // "textDocument/hover" => handle_hover(req).await,
        "textDocument/codeAction" => handle_code_action(req).await,
        _ => {
            warn!("unhandled request: {:?}", req);
            None
        }
    }
}

pub fn handle_notification(noti: Notification) -> Option<EspxResult> {
    return match noti.method.as_str() {
        "textDocument/didChange" => handle_didChange(noti),
        // "textDocument/didSave" => handle_didSave(noti),
        "textDocument/didOpen" => handle_didOpen(noti),
        s => {
            debug!("unhandled notification: {:?}", s);
            None
        }
    };
}

pub fn handle_other(msg: Message) -> Option<EspxResult> {
    warn!("unhandled message {:?}", msg);
    None
}

#[cfg(test)]
mod tests {
    use super::{handle_request, EspxResult, Request};
    use crate::htmx;
    use crate::text_store::{init_text_store, DOCUMENT_STORE};
    use lsp_types::Url;
    use std::collections::HashMap;
    use std::sync::Once;

    static SETUP: Once = Once::new();
    fn prepare_store(file: &str, content: &str) {
        SETUP.call_once(|| {
            htmx::init_hx_tags();
            init_text_store();
        });

        DOCUMENT_STORE
            .get()
            .expect("text store not initialized")
            .lock()
            .expect("text store mutex poisoned")
            .insert(
                Url::parse(file).unwrap(),
                crate::text_store::Document::from(content.to_string()),
            );
    }

    #[tokio::test]
    async fn handle_hover_it_presents_details_when_tag_value_is_under_cursor() {
        let file = "file:///detailstag.html";
        let content = r#"<div hx-target="next"></div>"#;

        prepare_store(file, content);

        let req = Request {
            id: 1.into(),
            method: "textDocument/hover".to_string(),
            params: serde_json::json!({
                "textDocument": {
                    "uri": file,
                },
                "position": {
                    "line": 0,
                    "character": 13
                }
            }),
        };

        let result = handle_request(req).await;

        assert!(result.is_some());
        match result {
            Some(EspxResult::PromptHover(h)) => {
                assert_eq!(h.id, 1.into());
                assert!(h.value.starts_with("hx-target"));
            }
            _ => {
                panic!("unexpected result: {:?}", result);
            }
        }
    }

    #[tokio::test]
    async fn handle_hover_it_presents_details_of_the_tag_name_when_is_under_cursor() {
        let file = "file:///detailstag.html";
        let content = r#"<div hx-target="next"></div>"#;

        prepare_store(file, content);

        let req = Request {
            id: 1.into(),
            method: "textDocument/hover".to_string(),
            params: serde_json::json!({
                "textDocument": {
                    "uri": file,
                },
                "position": {
                    "line": 0,
                    "character": 14
                }
            }),
        };

        let result = handle_request(req).await;

        assert!(result.is_some());
        match result {
            Some(EspxResult::PromptHover(h)) => {
                assert_eq!(h.id, 1.into());
                assert!(h.value.starts_with("hx-target"));
            }
            _ => {
                panic!("unexpected result: {:?}", result);
            }
        }
    }
}
