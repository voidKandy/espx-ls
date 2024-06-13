use super::{
    buffer_operations::{
        BufferOpChannelError, BufferOpChannelHandler, BufferOpChannelSender, BufferOperation,
    },
    error::HandleResult,
};
use crate::{handle::BufferOpChannelJoinHandle, state::SharedGlobalState};
use anyhow::anyhow;
use lsp_server::Request;
use lsp_types::{GotoDefinitionParams, HoverParams};
use tracing::{debug, error, info, warn};

/// Should probably create custom error types for this & notification
pub async fn handle_request(
    req: Request,
    state: SharedGlobalState,
) -> HandleResult<BufferOpChannelHandler> {
    error!("handle_request");
    let handle = BufferOpChannelHandler::new();

    let task_sender = handle.sender.clone();
    let _: BufferOpChannelJoinHandle = tokio::spawn(async move {
        match match req.method.as_str() {
            "textDocument/definition" => {
                handle_goto_definition(req, state, task_sender.clone()).await
            }
            "textDocument/hover" => handle_hover(req, state, task_sender.clone()).await,
            _ => {
                warn!("unhandled request: {:?}", req);
                Ok(())
            }
        } {
            Ok(_) => task_sender.send_finish().await.map_err(|err| err.into()),
            Err(err) => Err(err),
        }
    });
    return Ok(handle);
}

async fn handle_goto_definition(
    req: Request,
    mut state: SharedGlobalState,
    mut sender: BufferOpChannelSender,
) -> HandleResult<()> {
    let params = serde_json::from_value::<GotoDefinitionParams>(req.params)?;
    debug!("GOTO DEF REQUEST: {:?}", params);

    // let actual_pos = Position {
    //     line: params.text_document_position_params.position.line,
    //     // don't ask but i need to add 2 instead of 1 here.. idk
    //     character: params.text_document_position_params.position.character + 2,
    // };

    let mut w = state.get_write()?;

    // I dont love this .cloned() call, but I must borrow 'w' across this function
    if let Some(in_buffer_burn) = w
        .store
        .burns
        .get_burn_by_position(
            &params.text_document_position_params.text_document.uri,
            params.text_document_position_params.position,
        )
        .ok()
        .cloned()
    {
        in_buffer_burn
            .goto_definition_action(req.id, &mut sender, &mut w)
            .await
            .map_err(|err| {
                BufferOpChannelError::Undefined(anyhow!(
                    "Buffer burn goto action failed: {:?}",
                    err
                ))
            })?;
    }
    Ok(())
}

async fn handle_hover(
    req: Request,
    mut state: SharedGlobalState,
    mut sender: BufferOpChannelSender,
) -> HandleResult<()> {
    let params = serde_json::from_value::<HoverParams>(req.params)?;
    info!("GOT HOVER REQUEST: {:?}", params);

    let mut w = state.get_write()?;
    if let Some(echo_burn) = w
        .store
        .burns
        .get_burn_by_position(
            &params.text_document_position_params.text_document.uri,
            params.text_document_position_params.position, // actual_pos,
        )
        .ok()
    {
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
    mut sender: BufferOpChannelSender,
) -> HandleResult<()> {
    // let params: CodeActionParams = serde_json::from_value(req.params)?;
    // let response: Vec<CodeActionOrCommand> = {
    //     let mut vec: Vec<CodeActionOrCommand> = vec![];
    //     if params.range.end.line == params.range.start.line {
    //         // Each action will need to be handled
    //         let io_prompt_runes =
    //             UserQuickPrompt::all_from_action_params(params, &mut state.get_write()?.store);
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
