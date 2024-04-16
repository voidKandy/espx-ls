use crate::{
    cache::GlobalCache,
    config::GLOBAL_CONFIG,
    database::{
        chunks::{chunk_vec_content, DBDocumentChunk},
        docs::DBDocument,
        DB,
    },
    handle::diagnostics::EspxDiagnostic,
    state::SharedGlobalState,
};
use anyhow::anyhow;
use log::{debug, error, info};
use lsp_server::Notification;
use lsp_types::{DidChangeTextDocumentParams, DidSaveTextDocumentParams, TextDocumentItem};

use super::{
    error::EspxLsHandleError,
    operation_stream::{BufferOpStreamHandler, BufferOpStreamSender},
    BufferOperation, EspxLsResult,
};

#[derive(serde::Deserialize, Debug)]
struct TextDocumentOpen {
    #[serde(rename = "textDocument")]
    text_document: TextDocumentItem,
}

pub async fn handle_notification(
    noti: Notification,
    state: SharedGlobalState,
) -> EspxLsResult<BufferOpStreamHandler> {
    let handle = BufferOpStreamHandler::new();

    let task_sender = handle.sender.clone();
    let _: tokio::task::JoinHandle<EspxLsResult<()>> = tokio::spawn(async move {
        match noti.method.as_str() {
            "textDocument/didChange" => handle_didChange(noti, state, task_sender.clone()).await,
            "textDocument/didSave" => handle_didSave(noti, state, task_sender.clone()).await,
            "textDocument/didOpen" => handle_didOpen(noti, state, task_sender.clone()).await,
            s => {
                debug!("unhandled notification: {:?}", s);
                Ok(())
            }
        }
    });
    return Ok(handle);
}

#[allow(non_snake_case)]
async fn handle_didChange(
    noti: Notification,
    mut state: SharedGlobalState,
    mut sender: BufferOpStreamSender,
) -> EspxLsResult<()> {
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
) -> EspxLsResult<()> {
    let saved_text_doc: DidSaveTextDocumentParams =
        match serde_json::from_value::<DidSaveTextDocumentParams>(noti.params) {
            Err(err) => {
                error!("handle_didSave parsing params error : {:?}", err);
                return Ok(());
            }
            Ok(p) => p,
        };
    let text = saved_text_doc
        .text
        .ok_or(EspxLsHandleError::Undefined(anyhow!(
            "No text on didSave noti"
        )))?;
    let url = saved_text_doc.text_document.uri;
    let mut w = state.get_write()?;
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
) -> EspxLsResult<()> {
    let text_doc_item = serde_json::from_value::<TextDocumentOpen>(noti.params)?;
    let text = text_doc_item.text_document.text;
    let url = text_doc_item.text_document.uri;

    // Only update from didOpen noti when docs have free capacity.
    // Otherwise updates are done on save
    let r = state.get_read()?;
    let docs_already_full = r.cache.lru.docs_at_capacity().clone();
    drop(r);
    if !docs_already_full {
        // return Ok(Some(EspxDiagnostic::diagnose_document(url)?.into()));
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
    // if url.to_file_path().expect("Couldn't coerce url to filepath")
    //     == GLOBAL_CONFIG.paths.conversation_file_path
    // {
    // cache_mut.burns.push_listener_burns()?;
    // }

    sender
        .send_operation(EspxDiagnostic::diagnose_document(url, cache_mut)?.into())
        .await?;
    Ok(())

    // DONT DELETE!!
    // let db = DB
    //     .read()
    //     .map_err(|_| EspxHandleError::Undefined(anyhow!("Error reading DB")))?;
    // info!("DID OPEN GOT READ");
    // match db
    //     .get_doc_tuple_by_url(&url)
    //     .await
    //     .expect("Error querying database")
    // {
    //     None => {
    //         info!("DID OPEN NEEDS TO BUILD DB TUPLE");
    //         let tup = DBDocument::build_tuple(text.clone(), url.clone())
    //             .await
    //             .expect("Failed to build dbdoc tuple");
    //         info!("DID OPEN BUILT TUPLE");
    //         db.insert_document(&tup.0).await.unwrap();
    //         db.insert_chunks(&tup.1).await.unwrap();
    //     }
    //     Some((_, chunks)) => {
    //         info!("DID OPEN HAS TUPLE");
    //         if chunk_vec_content(&chunks) != text {
    //             info!("DID OPEN UPDATING");
    //             // THIS IS NOT A GOOD SOLUTION BECAUSE AT SOME POINT THE SUMMARY OF THE DOC
    //             // ENTRY WILL DEPRECATE
    //             // ALSO
    //             // A PATCH WOULD BE BETTER THAN JUST DELETING AND REPLACING ALL OF THE CHUNKS
    //             db.remove_chunks_by_url(&url)
    //                 .await
    //                 .expect("Could not remove chunks");
    //             let chunks = DBDocumentChunk::chunks_from_text(url.clone(), &text)
    //                 .await
    //                 .expect("Failed to get chunks from text");
    //             db.insert_chunks(&chunks)
    //                 .await
    //                 .expect("Could not insert chunks");
    //         }
    //     }
    // }
}
