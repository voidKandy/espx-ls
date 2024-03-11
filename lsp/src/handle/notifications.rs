use crate::{
    diagnostics::EspxDiagnostic,
    store::{set_doc_current, Document, GLOBAL_STORE},
};
use log::{debug, error};
use lsp_server::Notification;
use lsp_types::{DidSaveTextDocumentParams, TextDocumentItem};

use super::EspxResult;

#[derive(serde::Deserialize, Debug)]
struct TextDocumentOpen {
    #[serde(rename = "textDocument")]
    text_document: TextDocumentItem,
}

pub async fn handle_notification(noti: Notification) -> Option<EspxResult> {
    return match noti.method.as_str() {
        "textDocument/didChange" => handle_didChange(noti),
        "textDocument/didSave" => handle_didSave(noti).await,
        "textDocument/didOpen" => handle_didOpen(noti).await,
        s => {
            debug!("unhandled notification: {:?}", s);
            None
        }
    };
}

#[allow(non_snake_case)]
fn handle_didChange(noti: Notification) -> Option<EspxResult> {
    // let text_document_changes: DidChangeTextDocumentParams =
    //     serde_json::from_value(noti.params).ok()?;
    //
    // debug!("didChange Handle CHANGES: {:?}", text_document_changes);
    // if text_document_changes.content_changes.len() > 1 {
    //     debug!("BEWARE MULTIPLE CHANGES PASSED IN THIS NOTIFICATION");
    // }
    // let uri = text_document_changes.text_document.uri;
    // text_document_changes.content_changes.iter().for_each(|ch| {
    //     update_document_store_from_change_event(&uri, &ch).expect("Failed to process change");
    // });
    //
    None
}

#[allow(non_snake_case)]
async fn handle_didSave(noti: Notification) -> Option<EspxResult> {
    let saved_text_doc: DidSaveTextDocumentParams =
        match serde_json::from_value::<DidSaveTextDocumentParams>(noti.params) {
            Err(err) => {
                error!("handle_didSave parsing params error : {:?}", err);
                return None;
            }
            Ok(p) => p,
        };
    let text = saved_text_doc.text?;
    let uri = saved_text_doc.text_document.uri;
    set_doc_current(&uri, &text).await.ok()?;
    return Some(EspxResult::Diagnostics(EspxDiagnostic::diagnose_document(
        &text, uri,
    )));
}

#[allow(non_snake_case)]
async fn handle_didOpen(noti: Notification) -> Option<EspxResult> {
    let text_doc_item = serde_json::from_value::<TextDocumentOpen>(noti.params).ok()?;
    let uri = text_doc_item.text_document.uri;
    let doc = Document::new(uri.clone(), &text_doc_item.text_document.text);
    GLOBAL_STORE
        .get()
        .expect("text store not initialized")
        .lock()
        .expect("text store mutex poisoned")
        .documents
        .insert_or_update(doc, uri.clone())
        .ok()?;

    // let doc = get_text_document(&uri)
    //     .ok_or(anyhow::anyhow!("No document at that URL"))
    //     .ok()?;
    // update_agent_cache(doc, MessageRole::System, CopilotAgent::Assistant)
    //     .await
    //     .ok()?;
    // if let Some(mem_stream) = get_watcher_memory_stream().await.ok() {
    //     for mem in mem_stream.as_ref().into_iter() {
    //         update_agent_cache(
    //             mem.content.to_owned(),
    //             MessageRole::System,
    //             CopilotAgent::Assistant,
    //         )
    //         .await
    //         .ok()?;
    //     }
    // }

    debug!("didOpen Handle updated DOCUMENT_STORE");

    None
}
