use crate::{handle::diagnostics::EspxDiagnostic, state::SharedGlobalState};
use anyhow::anyhow;
use log::{debug, error, info};
use lsp_server::Notification;
use lsp_types::{DidChangeTextDocumentParams, DidSaveTextDocumentParams, TextDocumentItem};

use super::operation_stream::{
    BufferOpStreamError, BufferOpStreamHandler, BufferOpStreamResult, BufferOpStreamSender,
};

#[derive(serde::Deserialize, Debug)]
struct TextDocumentOpen {
    #[serde(rename = "textDocument")]
    text_document: TextDocumentItem,
}

pub async fn handle_notification(
    noti: Notification,
    state: SharedGlobalState,
) -> BufferOpStreamResult<BufferOpStreamHandler> {
    let handle = BufferOpStreamHandler::new();

    let task_sender = handle.sender.clone();
    let _: tokio::task::JoinHandle<BufferOpStreamResult<()>> = tokio::spawn(async move {
        match match noti.method.as_str() {
            "textDocument/didChange" => handle_didChange(noti, state, task_sender.clone()).await,
            "textDocument/didSave" => handle_didSave(noti, state, task_sender.clone()).await,
            "textDocument/didOpen" => handle_didOpen(noti, state, task_sender.clone()).await,
            s => {
                debug!("unhandled notification: {:?}", s);
                Ok(())
            }
        } {
            Ok(_) => task_sender.send_finish().await,
            Err(err) => Err(err.into()),
        }
    });
    return Ok(handle);
}

#[allow(non_snake_case)]
async fn handle_didChange(
    noti: Notification,
    mut state: SharedGlobalState,
    mut sender: BufferOpStreamSender,
) -> BufferOpStreamResult<()> {
    // THIS NO WORK
    // let mut s = state.get_write()?;
    // for change in text_document_changes.content_changes.into_iter() {
    //     s.cache.update_doc_changes(&change, url.clone())?;
    // }
    //
    let text_document_changes: DidChangeTextDocumentParams = serde_json::from_value(noti.params)?;
    let url = text_document_changes.text_document.uri;
    let change = &text_document_changes.content_changes[0];

    if let Some(mut w) = state.get_write().ok() {
        w.cache.lru.update_doc(&change.text, url.clone())?;
        let cache_mut = &mut w.cache;
        sender
            .send_operation(EspxDiagnostic::diagnose_document(url, cache_mut)?.into())
            .await?;
    }

    Ok(())
}

#[allow(non_snake_case)]
async fn handle_didSave(
    noti: Notification,
    mut state: SharedGlobalState,
    mut sender: BufferOpStreamSender,
) -> BufferOpStreamResult<()> {
    let saved_text_doc: DidSaveTextDocumentParams =
        serde_json::from_value::<DidSaveTextDocumentParams>(noti.params)?;
    let text = saved_text_doc
        .text
        .ok_or(BufferOpStreamError::Undefined(anyhow!(
            "No text on didSave noti"
        )))?;
    let url = saved_text_doc.text_document.uri;

    let mut w = state.get_write()?;

    if let Some(db) = &w.db {
        db.update_doc_store(&text, &url).await?;
    }

    w.cache.lru.update_doc(&text, url.clone())?;
    let cache_mut = &mut w.cache;
    sender
        .send_operation(EspxDiagnostic::diagnose_document(url, cache_mut)?.into())
        .await?;
    Ok(())
}

#[allow(non_snake_case)]
async fn handle_didOpen(
    noti: Notification,
    mut state: SharedGlobalState,
    mut sender: BufferOpStreamSender,
) -> BufferOpStreamResult<()> {
    let text_doc_item = serde_json::from_value::<TextDocumentOpen>(noti.params)?;
    let text = text_doc_item.text_document.text;
    let url = text_doc_item.text_document.uri;

    let r = state.get_read()?;

    if let Some(db) = &r.db {
        db.update_doc_store(&text, &url).await?;
    }

    // Only update from didOpen noti when docs have free capacity.
    // Otherwise updates are done on save
    let docs_already_full = r.cache.lru.docs_at_capacity().clone();
    drop(r);
    if !docs_already_full {
        info!("DOCS NOT FULL");
        state
            .get_write()?
            .cache
            .lru
            .update_doc(&text, url.clone())?;

        state
            .get_write()?
            .cache
            .lru
            .tell_listener_to_update_agent()?;
    }

    let cache_mut = &mut state.get_write()?.cache;

    sender
        .send_operation(EspxDiagnostic::diagnose_document(url, cache_mut)?.into())
        .await?;

    Ok(())
}
