pub mod error;
pub mod parsing;
pub mod user_actions;

use self::error::RuneError;
use crate::cache::GlobalCache;
use std::{collections::HashMap, time::SystemTime};

use anyhow::Result;
use crossbeam_channel::Sender;
use lsp_server::{Message, Notification};
use lsp_types::{
    ApplyWorkspaceEditParams, CodeAction, CodeActionParams, Command, ExecuteCommandParams,
    PublishDiagnosticsParams, Range, ShowMessageParams, TextEdit, Url, WorkspaceEdit,
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

#[derive(Debug)]
pub struct EspxActionExecutor {
    burn: RuneBufferBurn,
    workspace_edit: Option<ApplyWorkspaceEditParams>,
    message: Option<ShowMessageParams>,
}

#[derive(Debug, Clone)]
pub struct RuneBufferBurn {
    pub placeholder: (String, char),
    pub diagnostic_params: PublishDiagnosticsParams,
}

impl AsRef<RuneBufferBurn> for RuneBufferBurn {
    fn as_ref(&self) -> &RuneBufferBurn {
        &self
    }
}

type ExecutionReturn = (Sender<Message>, RuneBufferBurn);
impl EspxActionExecutor {
    pub fn url(&self) -> &Url {
        &self.burn.diagnostic_params.uri
    }
    pub fn execute(self, sender: Sender<Message>) -> Result<ExecutionReturn, RuneError> {
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

        Ok((sender, self.burn))
    }
}

// Gets executed:
// Turns into RuneBufferBurn object & will send workspace/applyEdit & worspace/showMessage when it does
// The edit sent to the text document in the event the trigger string is found
// The workspace message shown when the rune is activated
type DoActionReturn = (Option<ApplyWorkspaceEditParams>, Option<ShowMessageParams>);
pub trait ActionRune: ToCodeAction + Sized {
    fn all_from_text(text: &str, url: Url) -> Vec<Self>;
    fn try_from_execute_command_params(params: ExecuteCommandParams) -> Result<Self, RuneError>;
    // This is the string that the document is actually parsed for
    fn trigger_string() -> &'static str;
    // What actually happens when the rune is activated. Returns an executor which will send lsp
    async fn do_action(&self) -> Result<DoActionReturn, RuneError>;
    fn into_rune_burn(&self) -> RuneBufferBurn;
    fn into_executor(self, do_action_return: DoActionReturn) -> EspxActionExecutor {
        super::EspxActionExecutor {
            burn: self.into_rune_burn(),
            workspace_edit: do_action_return.0,
            message: do_action_return.1,
        }
    }
    fn all_from_action_params(params: CodeActionParams, cache: &mut GlobalCache) -> Vec<Self> {
        let text = cache
            .get_doc(&params.text_document.uri)
            .expect("Couldn't get doc from LRU");
        Self::all_from_text(&text, params.text_document.uri)
    }
}

impl RuneBufferBurn {
    pub fn generate_placeholder() -> char {
        let current_time = SystemTime::now();
        let possible = vec![
            '∀', '∁', '∂', '∃', '∄', '∅', '∆', '∇', '∈', '∉', '∊', '∋', '∌', '∍', '∎', '∏', '∐',
            '∑', '−', '∓', '∔', '∕', '∖', '∗', '∘', '∙', '√', '∛', '∜', '∝', '∞', '∟', '∠', '∡',
            '∢', '∣', '∤', '∥', '∦', '∧', '∨', '∩', '∪', '∫', '∬', '∭', '∮', '∯', '∰', '∱', '∲',
            '∳', '∴', '∵', '∶', '∷', '∸', '∹', '∺', '∻', '∼', '∽', '∾', '∿', '≀', '≁', '≂', '≃',
            '≄', '≅', '≆', '≇', '≈', '≉', '≊', '≋', '≌', '≍', '≎', '≏', '≐', '≑', '≒', '≓', '≔',
            '≕', '≖', '≗', '≘', '≙', '≚', '≛', '≜', '≝', '≞', '≟', '≠', '≡', '≢', '≣', '≤', '≥',
            '≦', '≧', '≨', '≩', '≪', '≫', '≬', '≭', '≮', '≯', '≰', '≱', '≲', '≳', '≴', '≵', '≶',
            '≷', '≸', '≹', '≺', '≻', '≼', '≽', '≾', '≿', '⊀', '⊁', '⊂', '⊃', '⊄', '⊅', '⊆', '⊇',
            '⊈', '⊉', '⊊', '⊋', '⊌', '⊍', '⊎', '⊏', '⊐', '⊑', '⊒', '⊓', '⊔', '⊕', '⊖', '⊗', '⊘',
            '⊙', '⊚', '⊛', '⊜', '⊝', '⊞', '⊟', '⊠', '⊡', '⊢', '⊣', '⊤', '⊥', '⊦', '⊧', '⊨', '⊩',
            '⊪', '⊫', '⊬', '⊭', '⊮', '⊯', '⊰', '⊱', '⊲', '⊳', '⊴', '⊵', '⊶', '⊷', '⊸', '⊹', '⊺',
            '⊻', '⊼', '⊽', '⊾', '⊿', '⋀', '⋁', '⋂', '⋃', '⋄', '⋅', '⋆', '⋇', '⋈', '⋉', '⋊', '⋋',
            '⋌', '⋍', '⋎', '⋏', '⋐', '⋑', '⋒', '⋓', '⋔', '⋕', '⋖', '⋗', '⋘', '⋙', '⋚', '⋛', '⋜',
            '⋝', '⋞', '⋟', '⋠', '⋡', '⋢', '⋣', '⋤', '⋥', '⋦', '⋧', '⋨', '⋩', '⋪', '⋫', '⋬', '⋭',
            '⋮', '⋯', '⋰', '⋱', '⋲', '⋳', '⋴', '⋵', '⋶', '⋷', '⋸', '⋹', '⋺', '⋻', '⋼', '⋽', '⋾',
            '⋿',
        ];

        let rand_indx = current_time.elapsed().unwrap().as_secs() as usize % (possible.len() - 1);
        possible[rand_indx]
    }
    /// Burn into document
    /// This entails:
    /// Editing the document to include the placeholder
    /// (Should be included on every save until the user removes the burn with a code action)
    /// Ensuring the burn is in the cache
    pub fn burn_into_cache(
        self,
        url: Url,
        sender: Sender<Message>,
        cache: &mut GlobalCache,
    ) -> Result<Sender<Message>> {
        sender.send(Message::Notification(Notification {
            method: "workspace/applyEdit".to_string(),
            params: serde_json::to_value(self.workspace_edit())?,
        }))?;
        cache.save_rune(url, self)?;
        Ok(sender)
    }

    fn range(&self) -> Range {
        self.diagnostic_params.diagnostics[0].range
    }

    fn workspace_edit(&self) -> ApplyWorkspaceEditParams {
        let mut changes = HashMap::new();
        let textedit = TextEdit {
            range: self.range(),
            new_text: format!("{}{}\n", self.placeholder.0, self.placeholder.1),
        };

        changes.insert(self.diagnostic_params.uri.clone(), vec![textedit]);

        let edit = WorkspaceEdit {
            changes: Some(changes),
            ..Default::default()
        };

        ApplyWorkspaceEditParams {
            label: Some(format!(
                "Insert rune with placeholder: {:?}",
                self.placeholder.1
            )),
            edit,
        }
    }
}
