use crate::{
    cache::GlobalCache,
    database::{
        chunks::{chunk_vec_content, DBDocumentChunk},
        docs::DBDocument,
        DB,
    },
    handle::diagnostics::EspxDiagnostic,
    state::{GlobalState, SharedGlobalState},
};
use anyhow::anyhow;
use log::{debug, error, info};
use lsp_server::Notification;
use lsp_types::{DidChangeTextDocumentParams, DidSaveTextDocumentParams, TextDocumentItem};

use super::{error::EspxHandleError, BufferOperation, EspxResult};

#[derive(serde::Deserialize, Debug)]
struct TextDocumentOpen {
    #[serde(rename = "textDocument")]
    text_document: TextDocumentItem,
}

pub async fn handle_notification(
    noti: Notification,
    state: SharedGlobalState,
) -> EspxResult<Option<BufferOperation>> {
    return match noti.method.as_str() {
        "textDocument/didChange" => handle_didChange(noti, state),
        "textDocument/didSave" => handle_didSave(noti, state).await,
        "textDocument/didOpen" => handle_didOpen(noti, state).await,
        s => {
            debug!("unhandled notification: {:?}", s);
            Ok(None)
        }
    };
}

#[allow(non_snake_case)]
fn handle_didChange(
    noti: Notification,
    mut state: SharedGlobalState,
) -> EspxResult<Option<BufferOperation>> {
    let text_document_changes: DidChangeTextDocumentParams = serde_json::from_value(noti.params)?;

    let url = text_document_changes.text_document.uri;

    // THIS NO WORK
    // let mut s = state.get_write()?;
    for change in text_document_changes.content_changes.into_iter() {
        // s.cache.update_doc_changes(&change, url.clone())?;
    }
    Ok(None)
}

#[allow(non_snake_case)]
async fn handle_didSave(
    noti: Notification,
    mut state: SharedGlobalState,
) -> EspxResult<Option<BufferOperation>> {
    let saved_text_doc: DidSaveTextDocumentParams =
        match serde_json::from_value::<DidSaveTextDocumentParams>(noti.params) {
            Err(err) => {
                error!("handle_didSave parsing params error : {:?}", err);
                return Ok(None);
            }
            Ok(p) => p,
        };
    let text = saved_text_doc
        .text
        .ok_or(EspxHandleError::Undefined(anyhow!(
            "No text on didSave noti"
        )))?;
    let url = saved_text_doc.text_document.uri;
    let mut s = state.get_write()?;
    s.cache.update_doc(&text, url.clone())?;
    let cache_mut = &mut s.cache;
    return Ok(Some(
        EspxDiagnostic::diagnose_document(url, cache_mut)?.into(),
    ));
}

#[allow(non_snake_case)]
async fn handle_didOpen(
    noti: Notification,
    mut state: SharedGlobalState,
) -> EspxResult<Option<BufferOperation>> {
    let text_doc_item = serde_json::from_value::<TextDocumentOpen>(noti.params)?;
    let text = text_doc_item.text_document.text;
    let url = text_doc_item.text_document.uri;

    // Only update from didOpen noti when docs have free capacity.
    // Otherwise updates are done on save
    let r = state.get_read()?;
    let docs_already_full = r.cache.docs_at_capacity().clone();
    drop(r);
    if !docs_already_full {
        // return Ok(Some(EspxDiagnostic::diagnose_document(url)?.into()));
        info!("DOCS NOT FULL");
        state.get_write()?.cache.update_doc(&text, url.clone())?;
    }

    let cache_mut = &mut state.get_write()?.cache;
    return Ok(Some(
        EspxDiagnostic::diagnose_document(url, cache_mut)?.into(),
    ));

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
