use log::{debug, info};
use lsp_types::{PublishDiagnosticsParams, Url};
pub mod error;
use crate::{
    burns::InBufferBurn,
    store::{error::StoreError, GlobalStore},
};
use error::DiagnosticError;

#[derive(Debug, Clone)]
pub enum EspxDiagnostic {
    ClearDiagnostics(Url),
    Publish(PublishDiagnosticsParams),
}

type DiagResult<T> = Result<T, DiagnosticError>;
impl EspxDiagnostic {
    pub fn diagnose_document(url: Url, store: &mut GlobalStore) -> DiagResult<Self> {
        info!("DIAGNOSING DOCUMENT");
        let mut all_diagnostics = vec![];
        let text = store.get_doc(&url).ok_or(StoreError::NotPresent)?;

        if let Some(actions) = InBufferBurn::all_actions_on_document(&text, url.clone()) {
            debug!("Diagnose document got actions: {:?}", actions);
            actions.into_iter().for_each(|b| {
                store
                    .burns
                    .save_burn(b.clone())
                    .expect("Failed to put burns in");
                all_diagnostics.push(b.burn.diagnostic());
            });
        }

        if let Some(echos) = store.burns.all_echos_on_doc(&url) {
            debug!("Diagnose document got echos: {:?}", echos);
            echos
                .into_iter()
                .for_each(|e| all_diagnostics.push(e.burn.diagnostic()))
        }

        info!("GOT DIAGNOSTICS: {:?}", all_diagnostics);
        match all_diagnostics.is_empty() {
            false => Ok(Self::Publish(PublishDiagnosticsParams {
                uri: url,
                diagnostics: all_diagnostics,
                version: None,
            })),
            true => Ok(Self::ClearDiagnostics(url)),
        }
    }
}
