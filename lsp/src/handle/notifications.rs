use super::{
    buffer_operations::{BufferOpChannelHandler, BufferOpChannelSender},
    error::HandleResult,
    BufferOpChannelJoinHandle,
};
use crate::{
    handle::{diagnostics::LspDiagnostic, error::HandleError},
    state::SharedGlobalState,
};
use anyhow::anyhow;
use lsp_server::Notification;
use lsp_types::{DidChangeTextDocumentParams, DidSaveTextDocumentParams, TextDocumentItem};
use tracing::{debug, info, warn};

#[derive(serde::Deserialize, Debug)]
struct TextDocumentOpen {
    #[serde(rename = "textDocument")]
    text_document: TextDocumentItem,
}

pub async fn handle_notification(
    noti: Notification,
    state: SharedGlobalState,
) -> HandleResult<BufferOpChannelHandler> {
    let handle = BufferOpChannelHandler::new();

    let task_sender = handle.sender.clone();
    let _: BufferOpChannelJoinHandle = tokio::spawn(async move {
        match match noti.method.as_str() {
            "textDocument/didChange" => handle_didChange(noti, state, task_sender.clone()).await,
            "textDocument/didSave" => handle_didSave(noti, state, task_sender.clone()).await,
            "textDocument/didOpen" => handle_didOpen(noti, state, task_sender.clone()).await,
            s => {
                debug!("unhandled notification: {:?}", s);
                Ok(())
            }
        } {
            Ok(_) => task_sender.send_finish().await.map_err(|err| err.into()),
            Err(err) => Err(err),
        }
    });
    return Ok(handle);
}

#[allow(non_snake_case)]
async fn handle_didChange(
    noti: Notification,
    mut state: SharedGlobalState,
    mut sender: BufferOpChannelSender,
) -> HandleResult<()> {
    let text_document_changes: DidChangeTextDocumentParams = serde_json::from_value(noti.params)?;
    let url = text_document_changes.text_document.uri;

    if text_document_changes.content_changes.len() > 1 {
        warn!("more than a single change recieved in notification");
        for change in text_document_changes.content_changes {
            if let Some(mut w) = state.get_write().ok() {
                w.store
                    .burns
                    .update_echos_from_change_event(&change, url.clone())?;

                w.store
                    .update_doc_from_lsp_change_notification(&change, url.clone())?;
                let store_mut = &mut w.store;
                sender
                    .send_operation(
                        LspDiagnostic::diagnose_document(url.clone(), store_mut)?.into(),
                    )
                    .await?;
            }
        }
    }

    Ok(())
}

#[allow(non_snake_case)]
async fn handle_didSave(
    noti: Notification,
    mut state: SharedGlobalState,
    mut sender: BufferOpChannelSender,
) -> HandleResult<()> {
    let saved_text_doc: DidSaveTextDocumentParams =
        serde_json::from_value::<DidSaveTextDocumentParams>(noti.params)?;
    let text = saved_text_doc
        .text
        .ok_or(HandleError::Undefined(anyhow!("No text on didSave noti")))?;
    let url = saved_text_doc.text_document.uri;

    let mut w = state.get_write()?;

    //     for placeholder in burns.iter().filter_map(|b| b.burn.echo_placeholder()) {
    //         if !text.contains(&placeholder) {
    //             debug!("TEXT NO LONGER CONTAINS: {}, REMOVING BURN", placeholder);
    //             placeholders_to_remove.push(placeholder)
    //         } else {
    //             debug!("TEXT STILL CONTAINS: {}", placeholder);
    //         }
    //     }
    // }

    // placeholders_to_remove
    //     .into_iter()
    //     .for_each(|p| w.store.burns.remove_echo_burn_by_placeholder(&url, &p));
    //
    // if let Some(db) = &w.store.db {
    //     db.update_doc_store(&text, &url).await?;
    // }

    w.store.update_doc(&text, url.clone());
    let store_mut = &mut w.store;
    sender
        .send_operation(LspDiagnostic::diagnose_document(url, store_mut)?.into())
        .await?;
    Ok(())
}

#[allow(non_snake_case)]
async fn handle_didOpen(
    noti: Notification,
    mut state: SharedGlobalState,
    mut sender: BufferOpChannelSender,
) -> HandleResult<()> {
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
        w.espx_env
            .updater
            .inner_write_lock()?
            .refresh_update_with_cache(&w.store)
            .await?;
    }

    let store_mut = &mut w.store;

    sender
        .send_operation(LspDiagnostic::diagnose_document(url, store_mut)?.into())
        .await?;

    Ok(())
}
