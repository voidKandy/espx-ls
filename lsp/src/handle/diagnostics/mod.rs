use log::info;
use lsp_types::{PublishDiagnosticsParams, Url};
pub mod error;

use crate::{burns::InBufferBurn, cache::GlobalCache};

use self::error::DiagnosticError;

// use super::actions::{InBufferAction, UserIoPrompt};

#[derive(Debug, Clone)]
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
        let burns = InBufferBurn::all_on_document(&text, url.clone());

        info!("BURNS IN BUFFER: {:?}", burns);

        burns
            .iter()
            .for_each(|ac| all_diagnostics.push(ac.clone().into()));

        // Still need to handle echos!
        // if let Some(burns) = cache.runes.all_burns_on_doc(&url).ok() {
        //     burns
        //         .into_iter()
        //         .for_each(|burn| all_diagnostics.push(burn.diagnostic_params.clone()));
        // }

        info!("GOT DIAGNOSTICS: {:?}", all_diagnostics);
        match all_diagnostics.is_empty() {
            false => Ok(Self::Publish(all_diagnostics)),
            true => Ok(Self::ClearDiagnostics(url)),
        }
    }
}
