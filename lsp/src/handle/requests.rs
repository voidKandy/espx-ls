use std::{collections::VecDeque, path::PathBuf};

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
    actions::{InBufferAction, ToCodeAction, UserIoPrompt},
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
        if let Some(executor) = prompt_action.into_executor().await.ok() {
            return Ok(Some(BufferOperation::CodeActionExecute(executor)));
        }
    }
    Ok(None)
}

// For making the role look 𝐍 𝐈 𝐂 𝐄
fn convert_ascii(str: &str, target: char) -> String {
    let start_code_point = target as u32;
    let str = str.to_lowercase();
    let mut chars = vec![' '];
    str.chars().for_each(|c| {
        let offset = c as u32 - 'a' as u32;
        chars.push(std::char::from_u32(start_code_point + offset).unwrap_or(c));
        chars.push(' ');
    });

    chars.into_iter().collect()
}

// For splitting the content of each message
fn split_message(message: &str, chunk_size: usize) -> Vec<String> {
    message
        .chars()
        .collect::<Vec<char>>()
        .chunks(chunk_size)
        .map(|chunk| chunk.iter().collect())
        .collect()
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
        .burns
        .get_burn_by_position(
            &params.text_document_position_params.text_document.uri,
            actual_pos,
        )
        .is_ok()
    {
        // CONVERSATION FILE WRITE
        if let Some(notis) = &ENV_HANDLE.get().unwrap().lock().unwrap().notifications {
            if let Some(EnvNotification::AgentStateUpdate { cache, .. }) =
                notis.read().await.find_by(|s| {
                    s.iter().rev().find(|env_noti| {
                        if let EnvNotification::AgentStateUpdate { agent_id, .. } = env_noti {
                            agent_id == InnerAgent::Assistant.id()
                        } else {
                            false
                        }
                    })
                })
            {
                let mut out_string_vec = vec![];
                for message in cache.as_ref().into_iter() {
                    debug!("CONVERSATION UPDATE ITERATION: {}", message);
                    let role_str = {
                        if let MessageRole::Other { alias, .. } = &message.role {
                            alias.to_string()
                        } else {
                            message.role.to_string()
                        }
                    };
                    let role_str = convert_ascii(&role_str, '𝐀');
                    debug!("CONVERSATION UPDATE PUSHING: {}", role_str);
                    out_string_vec.push(format!("# {}\n\n", &role_str));

                    for chunk in split_message(&message.content, 100) {
                        out_string_vec.push(chunk);
                        out_string_vec.push(String::from("\n"));
                    }
                }
                let content_to_write = out_string_vec.join("");
                std::fs::write(
                    GLOBAL_CONFIG.paths.conversation_file_path.clone(),
                    content_to_write,
                )
                .unwrap();
                debug!("CONVERSATION FILE WRITTEN");
            }
        }

        let path = &GLOBAL_CONFIG.paths.conversation_file_path;
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
        .burns
        .get_burn_by_position(
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
