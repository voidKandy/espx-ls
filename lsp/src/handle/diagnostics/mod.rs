use log::info;
use lsp_types::{PublishDiagnosticsParams, Url};
pub mod error;

use crate::cache::GlobalCache;

use self::error::DiagnosticError;

use super::actions::{InBufferAction, UserIoPrompt};

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
        let text = cache.lru.get_doc(&url)?;
        //  need to add more actions as they come
        let actions = UserIoPrompt::all_from_text(&text, url.clone());

        info!("GOT ACTIONS: {:?}", actions);
        actions
            .iter()
            .for_each(|ac| all_diagnostics.push(ac.as_diagnostics()));

        if let Some(burns) = cache.burns.all_burns_on_doc(&url).ok() {
            burns
                .into_iter()
                .for_each(|burn| all_diagnostics.push(burn.diagnostic_params.clone()));
        }

        info!("GOT DIAGNOSTICS: {:?}", all_diagnostics);
        match all_diagnostics.is_empty() {
            false => Ok(Self::Publish(all_diagnostics)),
            true => Ok(Self::ClearDiagnostics(url)),
        }
    }
}
