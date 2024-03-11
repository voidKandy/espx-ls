use crate::actions::{EspxAction, EspxActionExecutor};
use log::{debug, error, warn};
use lsp_server::Request;
use lsp_types::{CodeActionOrCommand, CodeActionParams, ExecuteCommandParams};

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
    let params = serde_json::from_value::<ExecuteCommandParams>(req.params).ok()?;
    debug!("COMMAND EXECUTION: {:?}", params);
    if let Some(ex) = EspxActionExecutor::try_from(params).ok() {
        return Some(EspxResult::CodeActionExecute(ex));
    }
    None
}

async fn handle_code_action_request(req: Request) -> Option<EspxResult> {
    let params: CodeActionParams = serde_json::from_value(req.params).ok()?;
    let all_actions = EspxAction::all_variants();
    let response: Vec<CodeActionOrCommand> = all_actions
        .into_iter()
        .filter_map(|a| a.try_from_params(&params))
        .flatten()
        .map(|builder| CodeActionOrCommand::CodeAction(builder.into()))
        .collect();

    Some(EspxResult::CodeActionRequest {
        response,
        id: req.id,
    })
}
