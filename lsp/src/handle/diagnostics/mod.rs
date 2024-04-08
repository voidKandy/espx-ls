use log::info;
use lsp_types::{PublishDiagnosticsParams, Url};
pub mod error;

use crate::cache::GlobalCache;

use self::error::DiagnosticError;

use super::runes::{user_actions::UserIoPrompt, ActionRune};

#[derive(Debug)]
pub enum EspxDiagnostic {
    ClearDiagnostics(Url),
    Publish(Vec<PublishDiagnosticsParams>),
}

type DiagResult<T> = Result<T, DiagnosticError>;
impl EspxDiagnostic {
    /// Now we need to bring back code action behavior for user prompts!
    pub fn diagnose_document(url: Url, cache: &mut GlobalCache) -> DiagResult<Self> {
        info!("DIAGNOSING DOCUMENT");
        let mut all_diagnostics = vec![];
        let text = cache.get_doc(&url)?;
        info!("GOT TEXT FOR {:?}", url.as_str());
        //  need to add more actions as they come
        let actions = UserIoPrompt::all_from_text(&text, url.clone());
        actions
            .iter()
            .for_each(|ac| all_diagnostics.push(ac.into_rune_burn().diagnostic_params));

        if let Some(burns) = cache.all_burns_on_doc(&url).ok() {
            burns
                .into_iter()
                .for_each(|burn| all_diagnostics.push(burn.diagnostic_params.clone()));
        }

        info!("GOT DIAGNOSTICS: {:?}", all_diagnostics);
        match all_diagnostics.is_empty() {
            true => Ok(Self::Publish(all_diagnostics)),
            false => Ok(Self::ClearDiagnostics(url)),
        }
    }
}
