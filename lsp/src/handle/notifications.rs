use super::{
    buffer_operations::{BufferOpChannelHandler, BufferOpChannelSender},
    error::HandleResult,
    BufferOpChannelJoinHandle,
};
use crate::{
    handle::{diagnostics::LspDiagnostic, error::HandleError},
    state::{
        burns::{
            Activation, Burn, MultiLineActivation, MultiLineVariant, SingleLineActivation,
            SingleLineVariant,
        },
        espx::AgentID,
        SharedGlobalState,
    },
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

#[tracing::instrument(name = "handle notification", skip_all)]
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
            Err(err) => return Err(err),
        }
    });
    return Ok(handle);
}

#[allow(non_snake_case)]
#[tracing::instrument(name = "didChange", skip_all)]
async fn handle_didChange(
    noti: Notification,
    mut state: SharedGlobalState,
    mut sender: BufferOpChannelSender,
) -> HandleResult<()> {
    let text_document_changes: DidChangeTextDocumentParams = serde_json::from_value(noti.params)?;
    let uri = text_document_changes.text_document.uri;

    let mut w = state.get_write()?;
    if text_document_changes.content_changes.len() > 1 {
        warn!("more than a single change recieved in notification");
        for change in text_document_changes.content_changes {
            w.store
                .update_doc_and_burns_from_lsp_change_notification(&change, uri.clone())?;
            // w.store.update_burns_on_doc(&uri).await?;
        }
    }

    sender
        .send_operation(LspDiagnostic::diagnose_document(uri, &mut w.store)?.into())
        .await?;
    Ok(())
}

#[allow(non_snake_case)]
#[tracing::instrument(name = "didSave", skip_all)]
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
    let uri = saved_text_doc.text_document.uri;

    let mut w = state.get_write()?;

    w.store.update_doc(&text, uri.clone());
    w.store.update_burns_on_doc(&uri)?;

    if let Some(burns_on_doc) = w.store.burns.take_burns_on_doc(&uri) {
        for mut b in burns_on_doc {
            if match b.activation {
                Activation::Multi(ref m) => {
                    #[allow(irrefutable_let_patterns)]
                    if let MultiLineVariant::LockChunkIntoContext = m.variant {
                        true
                    } else {
                        false
                    }
                }
                Activation::Single(ref s) => {
                    if let SingleLineVariant::LockDocIntoContext = s.variant {
                        true
                    } else {
                        false
                    }
                }
            } {
                debug!("activating burn: {:?}", &b);
                b.activate_with_agent(
                    uri.clone(),
                    None,
                    None,
                    &mut sender,
                    &mut w,
                    AgentID::Assistant,
                )
                .await?;
            }
            let _ = w.store.burns.insert_burn(uri.clone(), b);
        }
    }

    // w.store.try_update_database().await?;

    sender
        .send_operation(LspDiagnostic::diagnose_document(uri.clone(), &mut w.store)?.into())
        .await?;
    Ok(())
}

#[allow(non_snake_case)]
#[tracing::instrument(name = "didOpen", skip_all)]
async fn handle_didOpen(
    noti: Notification,
    mut state: SharedGlobalState,
    mut sender: BufferOpChannelSender,
) -> HandleResult<()> {
    let text_doc_item = serde_json::from_value::<TextDocumentOpen>(noti.params)?;
    let text = text_doc_item.text_document.text;
    let uri = text_doc_item.text_document.uri;

    let r = state.get_read()?;

    // Only update from didOpen noti when docs have free capacity.
    // Otherwise updates are done on save
    let docs_already_full = r.store.docs_at_capacity().clone();
    drop(r);

    let mut w = state.get_write()?;
    if !docs_already_full {
        w.store.update_doc(&text, uri.clone());
        w.refresh_agent_updater_with_cache().await?;
    }
    w.store.update_burns_on_doc(&uri)?;
    // w.store.try_update_from_database().await?;
    sender
        .send_operation(LspDiagnostic::diagnose_document(uri.clone(), &mut w.store)?.into())
        .await?;
    Ok(())
}
