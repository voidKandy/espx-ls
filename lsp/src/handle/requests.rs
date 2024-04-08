use log::{debug, error, warn};
use lsp_server::Request;
use lsp_types::{CodeActionOrCommand, CodeActionParams, ExecuteCommandParams};

use crate::state::SharedGlobalState;

use super::{
    runes::{user_actions::UserIoPrompt, ActionRune, ToCodeAction},
    BufferOperation, EspxResult,
};

/// Should probably create custom error types for this & notification
pub async fn handle_request(
    req: Request,
    state: SharedGlobalState,
) -> EspxResult<Option<BufferOperation>> {
    error!("handle_request");
    match req.method.as_str() {
        "workspace/executeCommand" => handle_execute_command(req).await,
        // "textDocument/hover" => handle_hover(req).await,
        "textDocument/codeAction" => handle_code_action_request(req, state).await,
        _ => {
            warn!("unhandled request: {:?}", req);
            Ok(None)
        }
    }
}

async fn handle_execute_command(
    req: Request,
    // state: SharedGlobalState,
) -> EspxResult<Option<BufferOperation>> {
    let params = serde_json::from_value::<ExecuteCommandParams>(req.params)?;
    debug!("COMMAND EXECUTION: {:?}", params);
    // Each action will need to be handled
    if let Some(prompt_action) = UserIoPrompt::try_from_execute_command_params(params).ok() {
        if let Some(ret) = prompt_action.do_action().await.ok() {
            return Ok(Some(BufferOperation::CodeActionExecute(
                prompt_action.into_executor(ret),
            )));
        }
    }
    Ok(None)
}

async fn handle_code_action_request(
    req: Request,
    mut state: SharedGlobalState,
) -> EspxResult<Option<BufferOperation>> {
    let params: CodeActionParams = serde_json::from_value(req.params)?;
    let response: Vec<CodeActionOrCommand> = {
        let mut vec: Vec<CodeActionOrCommand> = vec![];
        if params.range.end.line == params.range.start.line {
            // Each action will need to be handled
            let io_prompt_runes =
                UserIoPrompt::all_from_action_params(params, &mut state.get_write()?.cache);
            for rune in io_prompt_runes.into_iter() {
                vec.push(CodeActionOrCommand::CodeAction(rune.to_code_action()))
            }
        }
        vec
    };

    if response.is_empty() {
        return Ok(None);
    }

    Ok(Some(BufferOperation::CodeActionRequest {
        response,
        id: req.id,
    }))
}
