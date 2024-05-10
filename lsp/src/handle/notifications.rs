use super::operation_stream::{
    BufferOpStreamError, BufferOpStreamHandler, BufferOpStreamResult, BufferOpStreamSender,
};
use crate::{handle::diagnostics::EspxDiagnostic, state::SharedGlobalState};
use anyhow::anyhow;
use log::{debug, info};
use lsp_server::Notification;
use lsp_types::{DidChangeTextDocumentParams, DidSaveTextDocumentParams, TextDocumentItem};

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
    let text_document_changes: DidChangeTextDocumentParams = serde_json::from_value(noti.params)?;
    let url = text_document_changes.text_document.uri;
    let change = &text_document_changes.content_changes[0];

    if let Some(mut w) = state.get_write().ok() {
        w.store.update_doc(&change.text, url.clone());
        let store_mut = &mut w.store;
        sender
            .send_operation(EspxDiagnostic::diagnose_document(url, store_mut)?.into())
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

    let mut placeholders_to_remove = vec![];
    if let Some(burns) = w.store.burns.all_echos_on_doc(&url) {
        for placeholder in burns.iter().filter_map(|b| b.burn.echo_placeholder()) {
            if !text.contains(&placeholder) {
                debug!("TEXT NO LONGER CONTAINS: {}, REMOVING BURN", placeholder);
                placeholders_to_remove.push(placeholder)
            } else {
                debug!("TEXT STILL CONTAINS: {}", placeholder);
            }
        }
    }

    placeholders_to_remove
        .into_iter()
        .for_each(|p| w.store.burns.remove_echo_burn_by_placeholder(&url, &p));

    // if let Some(db) = &w.store.db {
    //     db.update_doc_store(&text, &url).await?;
    // }

    w.store.update_doc(&text, url.clone());
    let store_mut = &mut w.store;
    sender
        .send_operation(EspxDiagnostic::diagnose_document(url, store_mut)?.into())
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

    // if let Some(db) = &r.store.db {
    //     db.update_doc_store(&text, &url).await?;
    // }

    // Only update from didOpen noti when docs have free capacity.
    // Otherwise updates are done on save
    let docs_already_full = r.store.docs_at_capacity().clone();
    drop(r);

    let mut w = state.get_write()?;
    if !docs_already_full {
        info!("DOCS NOT FULL");
        w.store.update_doc(&text, url.clone());
        w.store.update_quick_agent();
    }

    let store_mut = &mut w.store;

    sender
        .send_operation(EspxDiagnostic::diagnose_document(url, store_mut)?.into())
        .await?;

    Ok(())
}
