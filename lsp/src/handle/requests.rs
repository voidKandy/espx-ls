use super::{
    buffer_operations::{BufferOpChannelHandler, BufferOpChannelSender, BufferOperation},
    error::HandleResult,
};
use crate::{
    handle::BufferOpChannelJoinHandle,
    state::{
        burns::{Burn, BurnActivation},
        espx::AgentID,
        SharedGlobalState,
    },
};
use anyhow::anyhow;
use lsp_server::Request;
use lsp_types::{GotoDefinitionParams, HoverParams, Position};
use tracing::{debug, error, info, warn};

/// Should probably create custom error types for this & notification
pub async fn handle_request(
    req: Request,
    state: SharedGlobalState,
) -> HandleResult<BufferOpChannelHandler> {
    let handle = BufferOpChannelHandler::new();

    let task_sender = handle.sender.clone();
    let _: BufferOpChannelJoinHandle = tokio::spawn(async move {
        match match req.method.as_str() {
            "textDocument/definition" => {
                handle_goto_definition(req, state, task_sender.clone()).await
            }
            "textDocument/hover" => handle_hover(req, state, task_sender.clone()).await,
            _ => {
                warn!("unhandled request method: {}", req.method);
                Ok(())
            }
        } {
            Ok(_) => task_sender.send_finish().await.map_err(|err| err.into()),
            Err(err) => Err(err),
        }
    });
    return Ok(handle);
}

#[tracing::instrument(name = "goto def", skip(state, sender))]
async fn handle_goto_definition(
    req: Request,
    mut state: SharedGlobalState,
    mut sender: BufferOpChannelSender,
) -> HandleResult<()> {
    let params = serde_json::from_value::<GotoDefinitionParams>(req.params)?;

    let actual_pos = Position {
        line: params.text_document_position_params.position.line,
        character: params.text_document_position_params.position.character + 1,
    };
    let uri = params.text_document_position_params.text_document.uri;

    let mut w = state.get_write()?;
    let mut agent = w
        .espx_env
        .agents
        .remove(&AgentID::Assistant)
        .expect("why no agent");

    if let Some(mut burn) = w.store.burns.take_burn(&uri, actual_pos.line) {
        match burn {
            BurnActivation::Single(ref mut single) => {
                single
                    .activate_on_document(
                        uri.clone(),
                        Some(req.id),
                        Some(actual_pos),
                        &mut sender,
                        &mut agent,
                        &mut w,
                    )
                    .await?;
            }
            _ => warn!("No multi line burns have any reason to have positional activation"), // BurnActivation::Multi(ref mut multi) => {
        }
        w.store.burns.insert_burn(uri, actual_pos.line, burn);
    }

    w.espx_env.agents.insert(AgentID::Assistant, agent);
    Ok(())
    // match activate_burn_at_position(req.id, actual_pos, &mut sender, uri, &mut w).await {
    //     Ok(()) => debug!("successfully activated burn"),
    //     Err(err) => error!("error activating burn: {:?}", err),
    // }

    // if let Some(burns_on_doc) = w.store.burns.read_burns_on_doc(&uri) {
    //      if let Some(burn_on_line) = burns_on_doc.get(&actual_pos.line) {
    //          burn_on_line.
    //      }
    //
    //  }

    // I dont love this .cloned() call, but I must borrow 'w' across this function
    // if let Some(in_buffer_burn) = w
    //     .store
    //     .burns
    //     .get_burn_by_position(
    //         &params.text_document_position_params.text_document.uri,
    //         params.text_document_position_params.position,
    //     )
    //     .ok()
    //     .cloned()
    // {
    //     in_buffer_burn
    //         .goto_definition_action(req.id, &mut sender, &mut w)
    //         .await
    //         .map_err(|err| {
    //             BufferOpChannelError::Undefined(anyhow!(
    //                 "Buffer burn goto action failed: {:?}",
    //                 err
    //             ))
    //         })?;
    // }
}

#[tracing::instrument(name = "hover", skip(state, sender))]
async fn handle_hover(
    req: Request,
    mut state: SharedGlobalState,
    mut sender: BufferOpChannelSender,
) -> HandleResult<()> {
    let params = serde_json::from_value::<HoverParams>(req.params)?;
    let uri = params.text_document_position_params.text_document.uri;
    let position = params.text_document_position_params.position;

    debug!(
        "got hover request on doc: {:?} as position: {:?}",
        uri.as_str(),
        position
    );

    let mut w = state.get_write()?;
    let text = w.store.get_doc(&uri)?;
    if let Some(burns_on_doc) = w.store.burns.read_burns_on_doc(&uri) {
        if let Some(burn_on_line) = burns_on_doc.get(&position.line) {
            match burn_on_line {
                BurnActivation::Single(single) => {
                    let (_, trigger_info) =
                        single.parse_for_user_input_and_trigger(position.line, &text)?;
                    if trigger_info.start <= position.character
                        && trigger_info.end >= position.character
                    {
                        if let Some(contents) = single.get_hover_contents().cloned() {
                            sender
                                .send_operation(BufferOperation::HoverResponse {
                                    contents,
                                    id: req.id,
                                })
                                .await?;
                        } else {
                            warn!("a burn matched that position but it did not have hover contents")
                        }
                    }
                }
                BurnActivation::Multi(_) => {
                    warn!("no multi line burns have any reason to display hover contents")
                }
            }
        }
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
