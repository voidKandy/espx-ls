use crate::{
    actions::{EspxAction, EspxActionExecutor},
    espx_env::{get_watcher_memory_stream, update_agent_cache, CopilotAgent},
    store::{
        get_text_document, set_doc_current, update_document_store_from_change_event, Document,
        GLOBAL_STORE,
    },
};
use espionox::environment::{
    agent::memory::MessageRole, dispatch::ThreadSafeStreamCompletionHandler,
};
use log::{debug, error, warn};
use lsp_server::{Message, Notification, Request, RequestId};
use lsp_types::{
    CodeActionOrCommand, CodeActionParams, CodeActionResponse, DidChangeTextDocumentParams,
    DidSaveTextDocumentParams, ExecuteCommandParams, TextDocumentItem,
};

#[derive(serde::Deserialize, Debug)]
struct TextDocumentOpen {
    #[serde(rename = "textDocument")]
    text_document: TextDocumentItem,
}

#[derive(Debug)]
pub struct EspxHoverResult {
    pub id: RequestId,
    pub value: String,
    pub handler: Option<ThreadSafeStreamCompletionHandler>,
}

#[derive(Debug)]
pub enum EspxResult {
    ShowMessage(String),
    ExecuteAction(EspxActionExecutor),
    CodeAction {
        response: CodeActionResponse,
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
    text_document_changes.content_changes.iter().for_each(|ch| {
        update_document_store_from_change_event(&uri, &ch).expect("Failed to process change");
    });

    None
}

#[allow(non_snake_case)]
fn handle_didSave(noti: Notification) -> Option<EspxResult> {
    let saved_text_doc = match serde_json::from_value::<DidSaveTextDocumentParams>(noti.params) {
        Ok(p) => p,
        Err(err) => {
            error!("handle_didSave parsing params error : {:?}", err);
            return None;
        }
    };
    set_doc_current(&saved_text_doc.text_document.uri, &saved_text_doc.text?).ok()?;

    None
}

#[allow(non_snake_case)]
async fn handle_didOpen(noti: Notification) -> Option<EspxResult> {
    let text_doc_item = serde_json::from_value::<TextDocumentOpen>(noti.params).ok()?;
    let uri = text_doc_item.text_document.uri;
    let doc = Document::from((&uri, text_doc_item.text_document.text.to_string()));
    GLOBAL_STORE
        .get()
        .expect("text store not initialized")
        .lock()
        .expect("text store mutex poisoned")
        .documents
        .insert_or_update(doc, uri.clone())
        .ok()?;

    let doc = get_text_document(&uri)
        .ok_or(anyhow::anyhow!("No document at that URL"))
        .ok()?;
    update_agent_cache(doc, MessageRole::System, CopilotAgent::Assistant)
        .await
        .ok()?;
    if let Some(mem_stream) = get_watcher_memory_stream().await.ok() {
        for mem in mem_stream.as_ref().into_iter() {
            update_agent_cache(
                mem.content.to_owned(),
                MessageRole::System,
                CopilotAgent::Assistant,
            )
            .await
            .ok()?;
        }
    }

    debug!("didOpen Handle updated DOCUMENT_STORE");

    None
}

async fn handle_code_action(req: Request) -> Option<EspxResult> {
    let params: CodeActionParams = serde_json::from_value(req.params).ok()?;
    debug!("CODE ACTION REQUEST: {:?}", params);
    let all_actions = EspxAction::all_variants();
    let response = all_actions
        .into_iter()
        .filter_map(|a| a.try_from_params(&params))
        .map(|espx_ac| CodeActionOrCommand::CodeAction(espx_ac.into()))
        .collect();

    Some(EspxResult::CodeAction {
        response,
        id: req.id,
    })
}

async fn handle_execute_command(req: Request) -> Option<EspxResult> {
    let params = serde_json::from_value::<ExecuteCommandParams>(req.params).ok()?;
    debug!("COMMAND EXECUTION: {:?}", params);
    if let Some(ex) = EspxActionExecutor::try_from(params).ok() {
        return Some(EspxResult::ExecuteAction(ex));
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

pub async fn handle_notification(noti: Notification) -> Option<EspxResult> {
    return match noti.method.as_str() {
        "textDocument/didChange" => handle_didChange(noti),
        "textDocument/didSave" => handle_didSave(noti),
        "textDocument/didOpen" => handle_didOpen(noti).await,
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
    use crate::store::{init_store, GLOBAL_STORE};
    use lsp_types::Url;
    use std::collections::HashMap;
    use std::sync::Once;

    static SETUP: Once = Once::new();
    fn prepare_store(file: &str, content: &str) {
        SETUP.call_once(|| {
            init_store();
        });
        let uri = Url::parse(file).unwrap();
        // GLOBAL_STORE
        //     .get()
        //     .expect("text store not initialized")
        //     .lock()
        //     .expect("text store mutex poisoned")
        //     .insert(
        //         uri.clone(),
        //         crate::store::Document::from((&uri, content.to_string())),
        //     );
    }

    // #[tokio::test]
    // async fn handle_hover_it_presents_details_of_the_tag_name_when_is_under_cursor() {
    //     let file = "file:///detailstag.html";
    //     let content = r#"<div hx-target="next"></div>"#;
    //
    //     prepare_store(file, content);
    //
    //     let req = Request {
    //         id: 1.into(),
    //         method: "textDocument/hover".to_string(),
    //         params: serde_json::json!({
    //             "textDocument": {
    //                 "uri": file,
    //             },
    //             "position": {
    //                 "line": 0,
    //                 "character": 14
    //             }
    //         }),
    //     };
    //
    //     let result = handle_request(req).await;
    //
    //     assert!(result.is_some());
    //     match result {
    //         Some(EspxResult::PromptHover(h)) => {
    //             assert_eq!(h.id, 1.into());
    //             assert!(h.value.starts_with("hx-target"));
    //         }
    //         _ => {
    //             panic!("unexpected result: {:?}", result);
    //         }
    //     }
    // }
}
