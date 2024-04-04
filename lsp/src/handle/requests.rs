use log::{error, warn};
use lsp_server::Request;

use super::EspxResult;

pub async fn handle_request(req: Request) -> Option<EspxResult> {
    error!("handle_request");
    match req.method.as_str() {
        "workspace/executeCommand" => handle_execute_command(req).await,
        // "textDocument/hover" => handle_hover(req).await,
        "textDocument/codeAction" => handle_code_action_request(req).await,
        _ => {
            warn!("unhandled request: {:?}", req);
            None
        }
    }
}

async fn handle_execute_command(req: Request) -> Option<EspxResult> {
    // let params = serde_json::from_value::<ExecuteCommandParams>(req.params).ok()?;
    // debug!("COMMAND EXECUTION: {:?}", params);
    // if let Some(action) = UserAction::try_from(params).ok() {
    //     return Some(EspxResult::CodeActionExecute(action));
    // }
    None
}

async fn handle_code_action_request(req: Request) -> Option<EspxResult> {
    // let params: CodeActionParams = serde_json::from_value(req.params).ok()?;
    // let response: Vec<CodeActionOrCommand> = {
    //     let mut vec: Vec<CodeActionOrCommand> = vec![];
    //     let url = params.text_document.uri;
    //     if params.range.end.line == params.range.start.line {
    //         // Need to write a catch to add the document to the cache from the database if text is
    //         // not Some
    //         if let Some(text) = GLOBAL_CACHE.write().unwrap().lru.get_doc(&url) {
    //             // Any additional code action runes will need to be added here
    //             if let Some(runes) = <UserPromptRune as CodeActionRune>::all_from_text_document::<
    //                 UserPromptRune,
    //             >(&text, url.clone())
    //             {
    //                 runes.into_iter().for_each(|rune| {
    //                     vec.push(CodeActionOrCommand::CodeAction(
    //                         rune.code_action(url.clone()),
    //                     ))
    //                 })
    //             }
    //         }
    //     }
    //     vec
    // };

    // if response.is_empty() {
    return None;
    // }

    // Some(EspxResult::CodeActionRequest {
    //     response,
    //     id: req.id,
    // })
}
