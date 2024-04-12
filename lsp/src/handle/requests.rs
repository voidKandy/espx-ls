use std::path::{Path, PathBuf};

use log::{debug, error, info, warn};
use lsp_server::Request;
use lsp_types::{
    request::HoverRequest, CodeActionOrCommand, CodeActionParams, ExecuteCommandParams,
    GotoDefinitionParams, HoverParams, Position, Url,
};

use crate::state::SharedGlobalState;

use super::{
    runes::{user_actions::UserIoPrompt, ActionRune, ToCodeAction},
    BufferOperation, EspxLsResult,
};

/// Should probably create custom error types for this & notification
pub async fn handle_request(
    req: Request,
    state: SharedGlobalState,
) -> EspxLsResult<Option<BufferOperation>> {
    error!("handle_request");
    match req.method.as_str() {
        "workspace/executeCommand" => handle_execute_command(req).await,
        "textDocument/definition" => handle_goto_definition(req, state).await,
        "textDocument/hover" => handle_hover(req, state).await,
        "textDocument/codeAction" => handle_code_action_request(req, state).await,
        _ => {
            warn!("unhandled request: {:?}", req);
            Ok(None)
        }
    }
}

async fn handle_execute_command(req: Request) -> EspxLsResult<Option<BufferOperation>> {
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

async fn handle_goto_definition(
    req: Request,
    state: SharedGlobalState,
) -> EspxLsResult<Option<BufferOperation>> {
    let params = serde_json::from_value::<GotoDefinitionParams>(req.params)?;
    debug!("GOTO DEF REQUEST: {:?}", params);

    let actual_pos = Position {
        line: params.text_document_position_params.position.line,
        // don't ask but i need to add 2 instead of 1 here.. idk
        character: params.text_document_position_params.position.character + 2,
    };

    let r = state.get_read()?;
    if r.cache
        .get_hovered_burn(
            &params.text_document_position_params.text_document.uri,
            actual_pos,
        )
        .is_ok()
    {
        let mut path = std::env::current_dir().unwrap().canonicalize().unwrap();
        path.push(PathBuf::from(".espx-ls/conversation.md"));
        let path_str = format!("file:///{}", path.display().to_string());
        debug!("PATH STRING: [{}]", path_str);

        let uri = Url::parse(&path_str).expect("Failed to build LSP URL from tempfile path");
        let response = lsp_types::GotoDefinitionResponse::Scalar(lsp_types::Location {
            uri,
            range: lsp_types::Range::default(),
        });
        return Ok(Some(BufferOperation::GotoFile {
            id: req.id,
            response,
        }));
    }
    Ok(None)
}

async fn handle_hover(
    req: Request,
    state: SharedGlobalState,
) -> EspxLsResult<Option<BufferOperation>> {
    let params = serde_json::from_value::<HoverParams>(req.params)?;
    info!("GOT HOVER REQUEST: {:?}", params);
    let r = state.get_read()?;
    // The LSP 1 indexes characters in the text doc, so we will add one to each value in the position
    let actual_pos = Position {
        line: params.text_document_position_params.position.line,
        // don't ask but i need to add 2 instead of 1 here.. idk
        character: params.text_document_position_params.position.character + 2,
    };
    if let Some(hover_contents) = r
        .cache
        .get_hovered_burn(
            &params.text_document_position_params.text_document.uri,
            actual_pos,
        )
        .ok()
    {
        return Ok(Some(BufferOperation::HoverResponse {
            contents: hover_contents,
            id: req.id,
        }));
    }

    Ok(None)
}

async fn handle_code_action_request(
    req: Request,
    mut state: SharedGlobalState,
) -> EspxLsResult<Option<BufferOperation>> {
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
