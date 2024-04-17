use log::{debug, info};
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

        if let Some(actions) = InBufferBurn::all_actions_on_document(&text, url.clone()) {
            debug!("Diagnose document got actions: {:?}", actions);
            actions.into_iter().for_each(|b| {
                cache
                    .burns
                    .save_burn(b.clone())
                    .expect("Failed to put burns in");
                all_diagnostics.push(b.into());
            });
        }

        if let Some(echos) = cache.burns.all_echos_on_doc(&url) {
            debug!("Diagnose document got echos: {:?}", echos);
            echos
                .into_iter()
                .for_each(|e| all_diagnostics.push(e.into()))
        }

        info!("GOT DIAGNOSTICS: {:?}", all_diagnostics);
        match all_diagnostics.is_empty() {
            false => Ok(Self::Publish(all_diagnostics)),
            true => Ok(Self::ClearDiagnostics(url)),
        }
    }
}
