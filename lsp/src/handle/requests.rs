use super::responses::code_actions::{
    EspxCodeActionBuilder, EspxCodeActionExecutor, EspxCodeActionVariant,
};
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
    if let Some(ex) = EspxCodeActionExecutor::try_from(params).ok() {
        return Some(EspxResult::CodeActionExecute(ex));
    }
    None
}

async fn handle_code_action_request(req: Request) -> Option<EspxResult> {
    let params: CodeActionParams = serde_json::from_value(req.params).ok()?;
    let all_action_variants = EspxCodeActionVariant::all_variants();
    let response: Vec<CodeActionOrCommand> = {
        let mut vec = vec![];
        for variant in all_action_variants.into_iter() {
            if let Some(action_builders) =
                EspxCodeActionBuilder::all_from_lsp_params(&params, &variant)
            {
                for builder in action_builders.into_iter() {
                    vec.push(CodeActionOrCommand::CodeAction(builder.into()));
                }
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
