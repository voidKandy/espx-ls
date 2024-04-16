use anyhow::Result;
use crossbeam_channel::Sender;
use lsp_server::{Message, Notification};
use lsp_types::Url;
use rand::Rng;

use crate::{burns::BufferBurn, cache::GlobalCache};

#[derive(Debug, Clone)]
pub struct ActionResponseBurn(BufferBurn);

impl From<BufferBurn> for ActionResponseBurn {
    fn from(value: BufferBurn) -> Self {
        Self(value)
    }
}

impl ActionResponseBurn {
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

    /// Burn into document
    /// This entails:
    /// Editing the document to include the placeholder
    /// (Should be included on every save until the user removes the burn with a code action)
    /// Ensuring the burn is in the cache
    pub fn burn_into_cache(
        self,
        sender: Sender<Message>,
        cache_mut: &mut GlobalCache,
    ) -> Result<Sender<Message>> {
        sender.send(Message::Notification(Notification {
            method: "workspace/applyEdit".to_string(),
            params: serde_json::to_value(self.0.workspace_edit())?,
        }))?;
        cache_mut.runes.save_burn(self.url().clone(), self.0)?;
        Ok(sender)
    }

    pub fn url(&self) -> &Url {
        &self.0.diagnostic_params.uri
    }
}
