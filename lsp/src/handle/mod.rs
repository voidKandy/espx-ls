use crate::{
    handle::{
        buffer_operations::{BufferOpChannelHandler, BufferOpChannelSender},
        error::HandleResult,
    },
    state::{
        burns::{Burn, BurnActivation},
        espx::AgentID,
        GlobalState,
    },
};
use anyhow::anyhow;
use lsp_server::{Message as LSPMessage, RequestId};
use lsp_types::{Position, Uri};
use tokio::sync::RwLockWriteGuard;
use tracing::warn;
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
    let mut burn = state_guard
        .store
        .burns
        .take_burn(&uri, position.line)
        .ok_or(anyhow!("no burn on that line"))?;

    match burn {
        BurnActivation::Single(ref mut single) => {
            single
                .activate_with_agent(
                    uri.clone(),
                    Some(request_id),
                    Some(position),
                    sender,
                    state_guard,
                    AgentID::Assistant,
                )
                .await?;
        }
        _ => warn!("No multi line burns have any reason to have positional activation"),
    }
    state_guard
        .store
        .burns
        .insert_burn(uri, position.line, burn);

    Ok(())
}

// pub async fn activate_burn(
//     burn: &mut BurnActivation,
//     sender: &mut BufferOpChannelSender,
//     store: &mut GlobalStore,
//     agent: &mut Agent,
//     user_input_info_opt: Option<TextAndCharRange>,
// ) -> HandleResult<()> {
//     if let Some(new_hover_content) = match &burn {
//         t @ BurnActivation::QuickPrompt { .. } | t @ BurnActivation::RagPrompt { .. } => {
//             debug!("DOING IO PROMPT ACTION");
//             let user_input_info = user_input_info_opt
//                 .expect("user info option should never be none with prompt variants");
//
//             // state_guard
//             //     .espx_env
//             //     .updater
//             //     .inner_write_lock()?
//             //     .refresh_update_with_cache(&state_guard.store)
//             //     .await?;
//             if !user_input_info.text.trim().is_empty() {
//                 agent.cache.push(Message::new_user(&user_input_info.text));
//
//                 let trigger = if let BurnActivation::RagPrompt { .. } = *t {
//                     Some("updater")
//                 } else {
//                     None
//                 };
//                 let mut response: ProviderStreamHandler =
//                     agent.do_action(stream_completion, (), trigger).await?;
//
//                 sender
//                     .send_work_done_report(Some("Got Stream Completion Handler"), None)
//                     .await
//                     .map_err(|err| HandleError::from(err))?;
//
//                 let mut whole_message = String::new();
//                 while let Some(status) = response.receive(agent).await {
//                     warn!("starting inference response loop");
//                     match status {
//                         CompletionStreamStatus::Working(token) => {
//                             warn!("got token: {}", token);
//                             whole_message.push_str(&token);
//                             sender
//                                 .send_work_done_report(Some(&token), None)
//                                 .await
//                                 .map_err(|err| BufferOpError::from(err))?;
//                         }
//                         CompletionStreamStatus::Finished => {
//                             warn!("finished");
//                             sender
//                                 .send_work_done_end(Some("Finished"))
//                                 .await
//                                 .map_err(|err| BufferOpError::from(err))?;
//                         }
//                     }
//                 }
//
//                 let message = ShowMessageParams {
//                     typ: MessageType::INFO,
//                     message: whole_message.clone(),
//                 };
//
//                 sender
//                     .send_operation(message.into())
//                     .await
//                     .map_err(|err| BufferOpError::from(err))?;
//
//                 let new_hover_content = HoverContents::Markup(lsp_types::MarkupContent {
//                     kind: MarkupKind::Markdown,
//                     value: [
//                         "# User prompt: ",
//                         &user_input_info.text,
//                         "# Assistant Response: ",
//                         &whole_message,
//                     ]
//                     .join("\n"),
//                 });
//
//                 Some(new_hover_content)
//             } else {
//                 None
//             }
//         }
//
//         BurnActivation::WalkProject { .. } => {
//             debug!("DOING WALK PROJECT ACTION");
//             let docs = walk_dir(PathBuf::from("."))?;
//             debug!("GOT DOCS: {:?}", docs);
//             let mut update_counter = 0;
//             for (i, (path, text)) in docs.iter().enumerate() {
//                 sender
//                     .send_work_done_report(
//                         Some(&format!("Walking {}", path.display())),
//                         Some((i as f32 / docs.len() as f32 * 100.0) as u32),
//                     )
//                     .await
//                     .map_err(|err| BufferOpError::from(err))?;
//
//                 let uri = Uri::from_str(&format!("file:///{}", path.display().to_string()))
//                     .expect("Failed to build uri");
//                 store.update_doc_store(&text, uri).await?;
//                 update_counter += 1;
//             }
//
//             // db.read_all_docs_to_cache().await?;
//             // state_guard.espx_env.updater.inner_write_lock()?.refresh_update_with_similar_database_chunks(db, prompt).await?;
//
//             // let content = self.typ.echo_content().to_string();
//             // let range = Range {
//             //     start: Position {
//             //         line: self.range.start.line,
//             //         character: self.replacement_text.len() as u32,
//             //     },
//             //     end: Position {
//             //         line: self.range.end.line,
//             //         character: self.replacement_text.len() as u32 + 1,
//             //     },
//             // };
//             sender
//                 .send_work_done_end(Some("Finished"))
//                 .await
//                 .map_err(|err| BufferOpError::from(err))?;
//             let new_hover_content = HoverContents::Markup(lsp_types::MarkupContent {
//                 kind: MarkupKind::Markdown,
//                 value: format!(
//                     "Finished walking project, added {:?} docs to database.",
//                     update_counter
//                 ),
//             });
//
//             Some(new_hover_content)
//         }
//     } {
//         burn.save_hover_contents(new_hover_content);
//     }
//     Ok(())
// }
