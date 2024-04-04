use lsp_types::{PublishDiagnosticsParams, Url};

use crate::cache::GLOBAL_CACHE;

#[derive(Debug)]
pub enum EspxDiagnostic {
    ClearDiagnostics(Url),
    Publish(Vec<PublishDiagnosticsParams>),
}

// impl TryInto<PublishDiagnosticsParams> for EspxCodeActionExecutor {
//     type Error = anyhow::Error;
//     fn try_into(self) -> Result<PublishDiagnosticsParams, Self::Error> {
//         match self {
//             EspxCodeActionExecutor::PromptOnLine { uri, line, .. } => {
//                 let range: lsp_types::Range = lsp_types::Range {
//                     start: Position { line, character: 0 },
//                     end: Position { line, character: 0 },
//                 };
//                 let message = String::from("Prompt code action available...");
//                 let diagnostic = Diagnostic {
//                     range,
//                     severity: Some(DiagnosticSeverity::HINT),
//                     message,
//                     ..Default::default()
//                 };
//                 Ok(PublishDiagnosticsParams {
//                     uri,
//                     diagnostics: vec![diagnostic],
//                     version: None,
//                 })
//             }
//         }
//     }
// }

impl EspxDiagnostic {
    /// Now we need to bring back code action behavior for user prompts!
    pub fn diagnose_document(url: Url) -> Self {
        if let Some(burn_map) = GLOBAL_CACHE.read().unwrap().runes.get(&url) {
            return Self::Publish(
                burn_map
                    .values()
                    .map(|burn| burn.diagnostic_params.clone())
                    .collect(),
            );
        }
        Self::ClearDiagnostics(url)
    }
}
