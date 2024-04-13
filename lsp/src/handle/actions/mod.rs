pub mod error;

pub(super) mod parsing;
pub mod response_burns;
pub mod user_io_prompt;

use self::{error::ActionError, response_burns::ActionResponseBurn};
use crate::cache::GlobalCache;
pub use user_io_prompt::UserIoPrompt;

use anyhow::Result;
use crossbeam_channel::Sender;
use lsp_server::{Message, Notification};
use lsp_types::{
    ApplyWorkspaceEditParams, CodeAction, CodeActionParams, Command, ExecuteCommandParams,
    PublishDiagnosticsParams, ShowMessageParams, Url, WorkspaceEdit,
};
use serde_json::Value;

pub trait ToCodeAction {
    fn command_id() -> String;

    fn title(&self) -> String;

    fn command_args(&self) -> Option<Vec<Value>>;

    fn workspace_edit(&self) -> Option<WorkspaceEdit>;

    fn to_code_action(&self) -> CodeAction;

    fn command(&self) -> Command {
        Command {
            title: Self::command_id(),
            command: Self::command_id(),
            arguments: self.command_args(),
        }
    }
}

// Gets executed:
// Turns into BufferBurn object & will send workspace/applyEdit & worspace/showMessage when it does
// The edit sent to the text document in the event the trigger string is found
// The workspace message shown when the rune is activated
type DoActionReturn = (
    ActionResponseBurn,
    Option<ApplyWorkspaceEditParams>,
    Option<ShowMessageParams>,
);

pub trait InBufferAction: ToCodeAction + Sized {
    fn all_from_text(text: &str, url: Url) -> Vec<Self>;
    fn try_from_execute_command_params(params: ExecuteCommandParams) -> Result<Self, ActionError>;
    // This is the string that the document is actually parsed for
    fn trigger_string() -> &'static str;
    /// When the action is within the document, we need to use diagnostics to tell the user there
    /// is an action available
    fn as_diagnostics(&self) -> PublishDiagnosticsParams;
    // What actually happens when the rune is activated. Returns an executor which will send lsp
    // messages & edits based on the returns of this function.
    async fn do_action(&self) -> Result<DoActionReturn, ActionError>;
    // Consumes self, does action and returns executor
    async fn into_executor(self) -> Result<EspxActionExecutor, ActionError> {
        let do_action_return = self.do_action().await?;
        Ok(super::EspxActionExecutor {
            action_response: do_action_return.0,
            workspace_edit: do_action_return.1,
            message: do_action_return.2,
        })
    }
    fn all_from_action_params(params: CodeActionParams, cache: &mut GlobalCache) -> Vec<Self> {
        let text = cache
            .lru
            .get_doc(&params.text_document.uri)
            .expect("Couldn't get doc from LRU");
        Self::all_from_text(&text, params.text_document.uri)
    }
}

#[derive(Debug)]
pub struct EspxActionExecutor {
    action_response: ActionResponseBurn,
    workspace_edit: Option<ApplyWorkspaceEditParams>,
    message: Option<ShowMessageParams>,
}

impl EspxActionExecutor {
    pub fn execute(
        self,
        sender: Sender<Message>,
        cache_mut: &mut GlobalCache,
    ) -> Result<Sender<Message>, ActionError> {
        if let Some(message) = self.message {
            sender.send(Message::Notification(Notification {
                method: "window/showMessage".to_string(),
                params: serde_json::to_value(message)?,
            }))?;
        }

        if let Some(edit) = self.workspace_edit {
            sender.send(Message::Notification(Notification {
                method: "workspace/applyEdit".to_string(),
                params: serde_json::to_value(edit)?,
            }))?;
        }

        let sender = self.action_response.burn_into_cache(sender, cache_mut)?;
        Ok(sender)
    }
}
