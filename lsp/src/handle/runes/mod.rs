pub mod error;
pub mod parsing;
pub mod user_actions;

use self::error::RuneError;
use crate::cache::GLOBAL_CACHE;
use std::{collections::HashMap, time::SystemTime};

use anyhow::Result;
use crossbeam_channel::Sender;
use lsp_server::{Message, Notification};
use lsp_types::{
    ApplyWorkspaceEditParams, PublishDiagnosticsParams, Range, ShowMessageParams, TextEdit, Url,
    WorkspaceEdit,
};

// pub fn burn_into_document(url: &Url, rune: impl BasicRune) -> Result<()> {
//     let burn = rune.buffer_burn(url.clone());
//
//     Ok(())
// }

pub fn generate_placeholder_for_doc(url: &Url) -> char {
    let current_time = SystemTime::now();
    let possible = vec![
        '∀', '∁', '∂', '∃', '∄', '∅', '∆', '∇', '∈', '∉', '∊', '∋', '∌', '∍', '∎', '∏', '∐', '∑',
        '−', '∓', '∔', '∕', '∖', '∗', '∘', '∙', '√', '∛', '∜', '∝', '∞', '∟', '∠', '∡', '∢', '∣',
        '∤', '∥', '∦', '∧', '∨', '∩', '∪', '∫', '∬', '∭', '∮', '∯', '∰', '∱', '∲', '∳', '∴', '∵',
        '∶', '∷', '∸', '∹', '∺', '∻', '∼', '∽', '∾', '∿', '≀', '≁', '≂', '≃', '≄', '≅', '≆', '≇',
        '≈', '≉', '≊', '≋', '≌', '≍', '≎', '≏', '≐', '≑', '≒', '≓', '≔', '≕', '≖', '≗', '≘', '≙',
        '≚', '≛', '≜', '≝', '≞', '≟', '≠', '≡', '≢', '≣', '≤', '≥', '≦', '≧', '≨', '≩', '≪', '≫',
        '≬', '≭', '≮', '≯', '≰', '≱', '≲', '≳', '≴', '≵', '≶', '≷', '≸', '≹', '≺', '≻', '≼', '≽',
        '≾', '≿', '⊀', '⊁', '⊂', '⊃', '⊄', '⊅', '⊆', '⊇', '⊈', '⊉', '⊊', '⊋', '⊌', '⊍', '⊎', '⊏',
        '⊐', '⊑', '⊒', '⊓', '⊔', '⊕', '⊖', '⊗', '⊘', '⊙', '⊚', '⊛', '⊜', '⊝', '⊞', '⊟', '⊠', '⊡',
        '⊢', '⊣', '⊤', '⊥', '⊦', '⊧', '⊨', '⊩', '⊪', '⊫', '⊬', '⊭', '⊮', '⊯', '⊰', '⊱', '⊲', '⊳',
        '⊴', '⊵', '⊶', '⊷', '⊸', '⊹', '⊺', '⊻', '⊼', '⊽', '⊾', '⊿', '⋀', '⋁', '⋂', '⋃', '⋄', '⋅',
        '⋆', '⋇', '⋈', '⋉', '⋊', '⋋', '⋌', '⋍', '⋎', '⋏', '⋐', '⋑', '⋒', '⋓', '⋔', '⋕', '⋖', '⋗',
        '⋘', '⋙', '⋚', '⋛', '⋜', '⋝', '⋞', '⋟', '⋠', '⋡', '⋢', '⋣', '⋤', '⋥', '⋦', '⋧', '⋨', '⋩',
        '⋪', '⋫', '⋬', '⋭', '⋮', '⋯', '⋰', '⋱', '⋲', '⋳', '⋴', '⋵', '⋶', '⋷', '⋸', '⋹', '⋺', '⋻',
        '⋼', '⋽', '⋾', '⋿',
    ];

    let mut rand_indx = current_time.elapsed().unwrap().as_secs() as usize % (possible.len() - 1);
    let mut p = possible[rand_indx];
    if let Some(doc_rune_map) = GLOBAL_CACHE.read().unwrap().runes.get(url) {
        let already_exist: Vec<&char> = doc_rune_map.keys().collect();
        loop {
            if !already_exist.contains(&&p) {
                p = possible[rand_indx];
                break;
            }

            rand_indx = current_time.elapsed().unwrap().as_secs() as usize % (possible.len() - 1);
        }
    }
    p
}

// Gets executed:
// Turns into RuneBufferBurn object & will send workspace/applyEdit & worspace/showMessage when it does
// The edit sent to the text document in the event the trigger string is found
// The workspace message shown when the rune is activated
type DoActionReturn = (Option<ApplyWorkspaceEditParams>, Option<ShowMessageParams>);
type ExecutionReturn = (Sender<Message>, RuneBufferBurn);
pub trait ActionRune: Sized {
    fn all_in_text(text: &str, url: &Url) -> Vec<Self>;
    // This is the string that the document is actually parsed for
    fn trigger_string() -> &'static str;
    fn into_buffer_burn(
        self,
        edit: Option<&ApplyWorkspaceEditParams>,
        message: Option<&ShowMessageParams>,
    ) -> RuneBufferBurn;
    // What actually happens when the rune is activated
    async fn do_action(&self) -> Result<DoActionReturn, RuneError>;
    async fn execute(self, sender: Sender<Message>) -> Result<ExecutionReturn, RuneError> {
        let action_return = self.do_action().await?;
        let burn = self.into_buffer_burn(action_return.0.as_ref(), action_return.1.as_ref());
        if let Some(message) = action_return.1 {
            sender.send(Message::Notification(Notification {
                method: "window/showMessage".to_string(),
                params: serde_json::to_value(message)?,
            }))?;
        }

        if let Some(edit) = action_return.0 {
            sender.send(Message::Notification(Notification {
                method: "workspace/applyEdit".to_string(),
                params: serde_json::to_value(edit)?,
            }))?;
        }

        Ok((sender, burn))
    }
}

#[derive(Debug)]
pub struct RuneBufferBurn {
    pub placeholder: (String, char),
    // hover_params: HoverParams,
    // hover_result: Hover,
    pub diagnostic_params: PublishDiagnosticsParams,
}

impl RuneBufferBurn {
    /// Burn into document
    /// This entails:
    /// Editing the document to include the placeholder
    /// (Should be included on every save until the user removes the burn with a code action)
    /// Ensuring the burn is in the cache
    fn burn(self, url: &Url, sender: Sender<Message>) -> Result<Sender<Message>> {
        sender.send(Message::Notification(Notification {
            method: "workspace/applyEdit".to_string(),
            params: serde_json::to_value(self.workspace_edit())?,
        }))?;

        let mut cache_write = GLOBAL_CACHE.write().unwrap();
        match cache_write.runes.get_mut(url) {
            Some(burn_map) => {
                burn_map.insert(self.placeholder.1, self);
            }
            None => {
                let mut burn_map = HashMap::new();
                burn_map.insert(self.placeholder.1, self);
                cache_write.runes.insert(url.clone(), burn_map);
            }
        }

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

// pub trait BasicRune: fmt::Debug + Send + Sync {
//     fn placeholder(&self) -> char;
//     fn diagnostics(&self) -> Vec<Diagnostic>;
//     fn publish_diagnostic_params(&self, url: Url) -> PublishDiagnosticsParams {
//         PublishDiagnosticsParams {
//             uri: url,
//             diagnostics: self.diagnostics(),
//             version: None,
//         }
//     }
//
//     fn buffer_burn(&self, url: Url) -> RuneBufferBurn {
//         let diagnostic_params = self.publish_diagnostic_params(url);
//         RuneBufferBurn { diagnostic_params }
//     }
//     // fn hover()
// }
//
// pub trait CodeActionRune: TryFrom<UserAction> + BasicRune {
//     fn title(&self) -> String;
//     fn edit(&self, url: Url) -> Option<WorkspaceEdit>;
//     fn command_id() -> String;
//     fn command_args(&self, url: &Url) -> Option<Vec<Value>>;
//     fn command(&self, url: &Url) -> Command {
//         Command {
//             title: self.title(),
//             command: Self::command_id(),
//             arguments: self.command_args(url),
//         }
//     }
//     fn code_action(&self, url: Url) -> CodeAction {
//         CodeAction {
//             title: self.title(),
//             command: Some(self.command(&url)),
//             edit: self.edit(url),
//             ..Default::default()
//         }
//     }
//
//     fn all_from_text_document<R>(text: &str, uri: Url) -> Option<Vec<R>>
//     where
//         R: CodeActionRune,
//     {
//         let mut all: Vec<R> = vec![];
//         let config = &GLOBAL_CONFIG;
//
//         let line_actions: Vec<UserAction> =
//             UserAction::all_actions_in_text(&config.user_actions, &text, uri);
//
//         for action in line_actions {
//             if let Some(rune) = action.try_into().ok() {
//                 all.push(rune);
//             }
//         }
//
//         match all.is_empty() {
//             true => None,
//             false => Some(all),
//         }
//     }
// }
