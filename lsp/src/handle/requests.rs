use std::{collections::VecDeque, path::PathBuf, sync::Arc};

use espionox::{agents::memory::MessageRole, environment::dispatch::EnvNotification};
use log::{debug, error, info, warn};
use lsp_server::Request;
use lsp_types::{
    CodeActionOrCommand, CodeActionParams, ExecuteCommandParams, GotoDefinitionParams, HoverParams,
    Position, Url,
};

use crate::{
    config::GLOBAL_CONFIG,
    espx_env::{agents::inner::InnerAgent, ENV_HANDLE},
    state::SharedGlobalState,
};

use super::{
    operation_stream::{BufferOpStreamHandler, BufferOpStreamResult, BufferOpStreamSender},
    BufferOperation, EspxLsResult,
};

/// Should probably create custom error types for this & notification
pub async fn handle_request(
    req: Request,
    state: SharedGlobalState,
) -> EspxLsResult<BufferOpStreamHandler> {
    error!("handle_request");
    let handle = BufferOpStreamHandler::new();

    let task_sender = handle.sender.clone();
    let _: tokio::task::JoinHandle<BufferOpStreamResult<()>> = tokio::spawn(async move {
        match match req.method.as_str() {
            // "workspace/executeCommand" => handle_execute_command(req).await,
            "textDocument/definition" => {
                handle_goto_definition(req, state, task_sender.clone()).await
            }
            "textDocument/hover" => handle_hover(req, state, task_sender.clone()).await,
            // "textDocument/codeAction" => handle_code_action_request(req, state).await,
            _ => {
                warn!("unhandled request: {:?}", req);
                Ok(())
            }
        } {
            Ok(_) => task_sender.send_finish().await,
            Err(err) => Err(err.into()),
        }
    });
    return Ok(handle);
}

async fn handle_execute_command(req: Request) -> EspxLsResult<Option<BufferOperation>> {
    // let params = serde_json::from_value::<ExecuteCommandParams>(req.params)?;
    // debug!("COMMAND EXECUTION: {:?}", params);
    // if let Some(prompt_action) = UserIoPrompt::try_from_execute_command_params(params).ok() {
    //     if let Some(executor) = prompt_action.into_executor().await.ok() {
    //         return Ok(Some(BufferOperation::CodeActionExecute(executor)));
    //     }
    // }
    Ok(None)
}
async fn handle_goto_definition(
    req: Request,
    mut state: SharedGlobalState,
    mut sender: BufferOpStreamSender,
) -> BufferOpStreamResult<()> {
    let params = serde_json::from_value::<GotoDefinitionParams>(req.params)?;
    debug!("GOTO DEF REQUEST: {:?}", params);

    let actual_pos = Position {
        line: params.text_document_position_params.position.line,
        // don't ask but i need to add 2 instead of 1 here.. idk
        character: params.text_document_position_params.position.character + 2,
    };

    let mut w = state.get_write()?;
    if let Some(in_buffer_burn) = w
        .cache
        .burns
        .get_burn_by_position(
            &params.text_document_position_params.text_document.uri,
            actual_pos,
        )
        .ok()
    {
        // in_buffer_burn.goto_definition_action(sender)
    }
    Ok(())
}

async fn handle_hover(
    req: Request,
    mut state: SharedGlobalState,
    mut sender: BufferOpStreamSender,
) -> BufferOpStreamResult<()> {
    let params = serde_json::from_value::<HoverParams>(req.params)?;
    info!("GOT HOVER REQUEST: {:?}", params);
    let mut w = state.get_write()?;
    // The LSP 1 indexes characters in the text doc, so we will add one to each value in the position
    let actual_pos = Position {
        line: params.text_document_position_params.position.line,
        // don't ask but i need to add 2 instead of 1 here.. idk
        character: params.text_document_position_params.position.character + 2,
    };
    if let Some(echo_burn) = w
        .cache
        .burns
        .get_burn_by_position(
            &params.text_document_position_params.text_document.uri,
            actual_pos,
        )
        .ok()
    {
        // LOGS SUGGEST RECEIVER WAS DROPPED
        sender
            .send_operation(BufferOperation::HoverResponse {
                contents: echo_burn.burn.hover_contents().unwrap(),
                id: req.id,
            })
            .await?;
    }

    Ok(())
}

async fn handle_code_action_request(
    req: Request,
    mut state: SharedGlobalState,
    mut sender: BufferOpStreamSender,
) -> BufferOpStreamResult<()> {
    // let params: CodeActionParams = serde_json::from_value(req.params)?;
    // let response: Vec<CodeActionOrCommand> = {
    //     let mut vec: Vec<CodeActionOrCommand> = vec![];
    //     if params.range.end.line == params.range.start.line {
    //         // Each action will need to be handled
    //         let io_prompt_runes =
    //             UserIoPrompt::all_from_action_params(params, &mut state.get_write()?.cache);
    //         for rune in io_prompt_runes.into_iter() {
    //             vec.push(CodeActionOrCommand::CodeAction(rune.to_code_action()))
    //         }
    //     }
    //     vec
    // };
    //
    // if response.is_empty() {
    return Ok(());
    // }

    // Ok(Some(BufferOperation::CodeActionRequest {
    //     response,
    //     id: req.id,
    // }))
}
