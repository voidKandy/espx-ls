use crate::{
    handle::{
        buffer_operations::{BufferOpChannelHandler, BufferOpChannelSender, BufferOpError},
        error::{HandleError, HandleResult},
    },
    state::{
        burns::{BurnActivation, TextAndCharRange},
        espx::AgentID,
        store::{walk_dir, GlobalStore},
        GlobalState,
    },
};
use anyhow::anyhow;
use espionox::{
    language_models::completions::streaming::{CompletionStreamStatus, ProviderStreamHandler},
    prelude::*,
};
use lsp_server::{Message as LSPMessage, RequestId};
use lsp_types::{
    ApplyWorkspaceEditParams, HoverContents, MarkupKind, MessageType, Position, Range,
    ShowMessageParams, TextEdit, Uri, WorkspaceEdit,
};
use std::{collections::HashMap, path::PathBuf, str::FromStr};
use tokio::sync::RwLockWriteGuard;
use tracing::{debug, warn};

pub mod buffer_operations;
pub mod diagnostics;
pub mod error;
pub mod notifications;
pub mod requests;

pub fn handle_other(msg: LSPMessage) -> HandleResult<BufferOpChannelHandler> {
    warn!("unhandled message {:?}", msg);
    Ok(BufferOpChannelHandler::new())
}
pub type BufferOpChannelJoinHandle = tokio::task::JoinHandle<error::HandleResult<()>>;

pub(super) async fn activate_burn_at_position(
    request_id: RequestId,
    position: Position,
    sender: &mut BufferOpChannelSender,
    uri: Uri,
    state_guard: &mut RwLockWriteGuard<'_, GlobalState>,
) -> HandleResult<()> {
    let doc = state_guard.store.get_doc(&uri)?;

    let mut agent = state_guard
        .espx_env
        .agents
        .remove(&AgentID::Assistant)
        .expect("why no agent");

    let mut burn = state_guard
        .store
        .burns
        .take_burn(&uri, position.line)
        .ok_or(anyhow!("no burn on that line"))?;

    let op = burn.doing_action_notification();
    sender.send_operation(op).await?;
    let (user_input_info_opt, trigger_info) =
        burn.parse_for_user_input_and_trigger(position.line, &doc)?;

    let user_input_triggered_opt: Option<bool> = {
        if trigger_info.start <= position.character && trigger_info.end >= position.character {
            Some(false)
        } else {
            user_input_info_opt.as_ref().and_then(|info| {
                Some(info.start <= position.character && info.end >= position.character)
            })
        }
    };

    if let Some(trigger_is_user_input) = user_input_triggered_opt {
        if !trigger_is_user_input {
            if let Some(op) = burn.goto_definition_on_trigger_response(request_id)? {
                sender.send_operation(op).await?;
            }
        } else {
            let end_char = match user_input_info_opt {
                Some(ref user_input_info) => user_input_info.end as u32,
                None => trigger_info.end as u32,
            };
            let edit_range = Range {
                start: Position {
                    line: position.line as u32,
                    character: trigger_info.start as u32,
                },
                end: Position {
                    line: position.line as u32,
                    character: end_char,
                },
            };

            let mut changes = HashMap::new();
            changes.insert(
                uri.clone(),
                vec![TextEdit {
                    range: edit_range,
                    new_text: burn.echo_content().to_owned(),
                }],
            );

            let edit_params = ApplyWorkspaceEditParams {
                label: None,
                edit: WorkspaceEdit {
                    changes: Some(changes),
                    ..Default::default()
                },
            };

            sender.send_operation(edit_params.into()).await?;
            activate_burn(
                &mut burn,
                sender,
                &mut state_guard.store,
                &mut agent,
                user_input_info_opt,
            )
            .await?;
        }
    }

    // if let BurnActivation::RagPrompt { .. } = burn {
    //     if let Some(db) = &state_guard.store.db {
    //         state_guard
    //             .espx_env
    //             .updater
    //             .inner_write_lock()?
    //             .refresh_update_with_similar_database_chunks(
    //                 &db.client,
    //                 &
    //                     .user_input
    //                     .as_ref()
    //                     .expect("Why is there no user input?"),
    //                 sender,
    //             )
    //             .await?;
    //     }
    // }

    state_guard
        .store
        .burns
        .insert_burn(uri, position.line, burn);

    state_guard
        .espx_env
        .agents
        .insert(AgentID::Assistant, agent);
    Ok(())
}

pub async fn activate_burn(
    burn: &mut BurnActivation,
    sender: &mut BufferOpChannelSender,
    store: &mut GlobalStore,
    agent: &mut Agent,
    user_input_info_opt: Option<TextAndCharRange>,
) -> HandleResult<()> {
    if let Some(new_hover_content) = match &burn {
        t @ BurnActivation::QuickPrompt { .. } | t @ BurnActivation::RagPrompt { .. } => {
            debug!("DOING IO PROMPT ACTION");
            let user_input_info = user_input_info_opt
                .expect("user info option should never be none with prompt variants");

            // state_guard
            //     .espx_env
            //     .updater
            //     .inner_write_lock()?
            //     .refresh_update_with_cache(&state_guard.store)
            //     .await?;
            if !user_input_info.text.trim().is_empty() {
                agent.cache.push(Message::new_user(&user_input_info.text));

                let trigger = if let BurnActivation::RagPrompt { .. } = *t {
                    Some("updater")
                } else {
                    None
                };
                let mut response: ProviderStreamHandler =
                    agent.do_action(stream_completion, (), trigger).await?;

                sender
                    .send_work_done_report(Some("Got Stream Completion Handler"), None)
                    .await
                    .map_err(|err| HandleError::from(err))?;

                let mut whole_message = String::new();
                while let Some(status) = response.receive(agent).await {
                    warn!("starting inference response loop");
                    match status {
                        CompletionStreamStatus::Working(token) => {
                            warn!("got token: {}", token);
                            whole_message.push_str(&token);
                            sender
                                .send_work_done_report(Some(&token), None)
                                .await
                                .map_err(|err| BufferOpError::from(err))?;
                        }
                        CompletionStreamStatus::Finished => {
                            warn!("finished");
                            sender
                                .send_work_done_end(Some("Finished"))
                                .await
                                .map_err(|err| BufferOpError::from(err))?;
                        }
                    }
                }

                let message = ShowMessageParams {
                    typ: MessageType::INFO,
                    message: whole_message.clone(),
                };

                sender
                    .send_operation(message.into())
                    .await
                    .map_err(|err| BufferOpError::from(err))?;

                let new_hover_content = HoverContents::Markup(lsp_types::MarkupContent {
                    kind: MarkupKind::Markdown,
                    value: [
                        "# User prompt: ",
                        &user_input_info.text,
                        "# Assistant Response: ",
                        &whole_message,
                    ]
                    .join("\n"),
                });

                Some(new_hover_content)
            } else {
                None
            }
        }

        BurnActivation::WalkProject { .. } => {
            debug!("DOING WALK PROJECT ACTION");
            let docs = walk_dir(PathBuf::from("."))?;
            debug!("GOT DOCS: {:?}", docs);
            let mut update_counter = 0;
            for (i, (path, text)) in docs.iter().enumerate() {
                sender
                    .send_work_done_report(
                        Some(&format!("Walking {}", path.display())),
                        Some((i as f32 / docs.len() as f32 * 100.0) as u32),
                    )
                    .await
                    .map_err(|err| BufferOpError::from(err))?;

                let uri = Uri::from_str(&format!("file:///{}", path.display().to_string()))
                    .expect("Failed to build uri");
                store.update_doc_store(&text, uri).await?;
                update_counter += 1;
            }

            // db.read_all_docs_to_cache().await?;
            // state_guard.espx_env.updater.inner_write_lock()?.refresh_update_with_similar_database_chunks(db, prompt).await?;

            // let content = self.typ.echo_content().to_string();
            // let range = Range {
            //     start: Position {
            //         line: self.range.start.line,
            //         character: self.replacement_text.len() as u32,
            //     },
            //     end: Position {
            //         line: self.range.end.line,
            //         character: self.replacement_text.len() as u32 + 1,
            //     },
            // };
            sender
                .send_work_done_end(Some("Finished"))
                .await
                .map_err(|err| BufferOpError::from(err))?;
            let new_hover_content = HoverContents::Markup(lsp_types::MarkupContent {
                kind: MarkupKind::Markdown,
                value: format!(
                    "Finished walking project, added {:?} docs to database.",
                    update_counter
                ),
            });

            Some(new_hover_content)
        }
    } {
        burn.save_hover_contents(new_hover_content);
    }
    Ok(())
}
