use crate::cache::GlobalCache;
use rand::Rng;
use std::collections::HashMap;

use anyhow::Result;
use crossbeam_channel::Sender;
use lsp_server::{Message, Notification};
use lsp_types::{
    ApplyWorkspaceEditParams, HoverContents, PublishDiagnosticsParams, Range, TextEdit, Url,
    WorkspaceEdit,
};

#[derive(Debug, Clone)]
pub struct BufferBurn {
    pub content: String,
    pub diagnostic_params: PublishDiagnosticsParams,
    pub hover_contents: HoverContents,
}

impl AsRef<BufferBurn> for BufferBurn {
    fn as_ref(&self) -> &BufferBurn {
        &self
    }
}

impl BufferBurn {
    pub fn generate_placeholder() -> String {
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
            '⊪', '⊫', '⊬', '⊭', '⊮', '⊯', '⊰', '⊱', '⊲', '⊳', '⊴', '⊵', '⊹', '⊺', '⊻', '⊼', '⊽',
            '⊾', '⊿', '⋀', '⋁', '⋂', '⋃', '⋄', '⋅', '⋆', '⋇', '⋈', '⋉', '⋊', '⋋', '⋌', '⋍', '⋎',
            '⋏', '⋐', '⋑', '⋒', '⋓', '⋔', '⋕', '⋖', '⋗', '⋘', '⋙', '⋚', '⋛', '⋜', '⋝', '⋞', '⋟',
            '⋠', '⋡', '⋢', '⋣', '⋤', '⋥', '⋦', '⋧', '⋨', '⋩', '⋪', '⋫', '⋬', '⋭', '⋮', '⋯', '⋰',
            '⋱', '⋲', '⋳', '⋴', '⋵', '⋶', '⋷', '⋸', '⋹', '⋺', '⋻', '⋽', '⋾', '⋿',
        ];

        // let rand_indx = current_time.elapsed().unwrap().as_secs() as usize % (possible.len() - 1);
        let index = rand::thread_rng().gen_range(0..possible.len());
        possible[index].to_string()
    }

    pub fn range(&self) -> Range {
        self.diagnostic_params.diagnostics[0].range
    }

    pub fn workspace_edit(&self) -> ApplyWorkspaceEditParams {
        let mut changes = HashMap::new();
        let textedit = TextEdit {
            range: self.range(),
            new_text: self.content.to_owned(),
        };

        changes.insert(self.diagnostic_params.uri.clone(), vec![textedit]);

        let edit = WorkspaceEdit {
            changes: Some(changes),
            ..Default::default()
        };

        ApplyWorkspaceEditParams {
            label: Some(format!("Insert rune: {}", self.content)),
            edit,
        }
    }
}
