use log::{debug, error, warn};
use lsp_server::Request;
use lsp_types::{CodeActionOrCommand, CodeActionParams, ExecuteCommandParams};

use super::{
    runes::{user_actions::UserIoPrompt, ActionRune, ToCodeAction},
    EspxResult,
};

/// Should probably create custom error types for this & notification
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
    let params = serde_json::from_value::<ExecuteCommandParams>(req.params).ok()?;
    debug!("COMMAND EXECUTION: {:?}", params);
    // Each action will need to be handled
    if let Some(prompt_action) = UserIoPrompt::try_from_execute_command_params(params).ok() {
        if let Some(ret) = prompt_action.do_action().await.ok() {
            return Some(EspxResult::CodeActionExecute(
                prompt_action.into_executor(ret).ok()?,
            ));
        }
    }
    None
}

async fn handle_code_action_request(req: Request) -> Option<EspxResult> {
    let params: CodeActionParams = serde_json::from_value(req.params).ok()?;
    let response: Vec<CodeActionOrCommand> = {
        let mut vec: Vec<CodeActionOrCommand> = vec![];
        if params.range.end.line == params.range.start.line {
            // Each action will need to be handled
            let io_prompt_runes = UserIoPrompt::all_from_action_params(params);
            for rune in io_prompt_runes.into_iter() {
                vec.push(CodeActionOrCommand::CodeAction(rune.to_code_action()))
            }
        }
        vec
    };

    if response.is_empty() {
        return None;
    }

    Some(EspxResult::CodeActionRequest {
        response,
        id: req.id,
    })
}
