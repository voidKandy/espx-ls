use crate::{
    cache::GLOBAL_CACHE,
    database::{
        chunks::{chunk_vec_content, DBDocumentChunk},
        docs::DBDocument,
        DB,
    },
    handle::diagnostics::EspxDiagnostic,
};
use log::{debug, error, info};
use lsp_server::Notification;
use lsp_types::{DidChangeTextDocumentParams, DidSaveTextDocumentParams, TextDocumentItem};

use super::{runes::ActionRune, EspxResult};

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
    let text_document_changes: DidChangeTextDocumentParams =
        serde_json::from_value(noti.params).ok()?;

    let mut cache = GLOBAL_CACHE.write().unwrap();
    let url = text_document_changes.text_document.uri;
    for change in text_document_changes.content_changes.into_iter() {
        match cache.lru.update_changes(&change, url.clone()) {
            Ok(_) => {
                info!("Cache succesfully updated")
            }
            Err(err) => {
                error!("Error updating cache: {:?}", err)
            }
        }
    }

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
    let url = saved_text_doc.text_document.uri;
    let mut cache = GLOBAL_CACHE.write().unwrap();
    cache.lru.update_doc(&text, url.clone());
    drop(cache);
    let diagnostic = EspxDiagnostic::diagnose_document(url);
    return Some(EspxResult::Diagnostics(diagnostic));
}

#[allow(non_snake_case)]
async fn handle_didOpen(noti: Notification) -> Option<EspxResult> {
    let text_doc_item = serde_json::from_value::<TextDocumentOpen>(noti.params).ok()?;
    //
    let text = text_doc_item.text_document.text;
    let url = text_doc_item.text_document.uri;

    let mut cache = GLOBAL_CACHE.write().unwrap();

    // Only update from didOpen noti when docs have free capacity.
    // Otherwise updates are done on save
    if !cache.lru.docs_at_capacity() {
        cache.lru.update_doc(&text, url.clone());
    }

    let db = DB.read().ok()?;
    info!("DID OPEN GOT READ");
    match db
        .get_doc_tuple_by_url(&url)
        .await
        .expect("Error querying database")
    {
        None => {
            info!("DID OPEN NEEDS TO BUILD DB TUPLE");
            let tup = DBDocument::build_tuple(text.clone(), url.clone())
                .await
                .expect("Failed to build dbdoc tuple");
            info!("DID OPEN BUILT TUPLE");
            db.insert_document(&tup.0).await.unwrap();
            db.insert_chunks(&tup.1).await.unwrap();
        }
        Some((_, chunks)) => {
            info!("DID OPEN HAS TUPLE");
            if chunk_vec_content(&chunks) != text {
                info!("DID OPEN UPDATING");
                // THIS IS NOT A GOOD SOLUTION BECAUSE AT SOME POINT THE SUMMARY OF THE DOC
                // ENTRY WILL DEPRECATE
                // ALSO
                // A PATCH WOULD BE BETTER THAN JUST DELETING AND REPLACING ALL OF THE CHUNKS
                db.remove_chunks_by_url(&url).await.unwrap();
                let chunks = DBDocumentChunk::chunks_from_text(url.clone(), &text)
                    .await
                    .unwrap();
                db.insert_chunks(&chunks).await.unwrap();
            }
        }
    }

    None
}
