// use crate::{
//     burns::{Burn, InBufferBurn},
//     store::{error::StoreError, GlobalStore},
// };
use anyhow::anyhow;
use lsp_types::{PublishDiagnosticsParams, Uri};
use tracing::{debug, info};

use crate::state::{
    burns::{Burn, InBufferBurn},
    store::GlobalStore,
};

#[derive(Debug, Clone)]
pub enum LspDiagnostic {
    ClearDiagnostics(Uri),
    Publish(PublishDiagnosticsParams),
}

impl LspDiagnostic {
    #[tracing::instrument(name = "diagnosing document", skip(store))]
    pub fn diagnose_document(url: Uri, store: &mut GlobalStore) -> anyhow::Result<Self> {
        let mut all_diagnostics = vec![];
        let text = store
            .get_doc(&url)
            .ok_or(anyhow!("no doc for: {:?}", url))?;

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

        if let Some(echos) = store.burns.all_in_buffer_burns_on_doc(&url, |b| {
            if let Burn::Echo(_) = b.burn {
                true
            } else {
                false
            }
        }) {
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
