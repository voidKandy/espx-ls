use lsp_types::{Diagnostic, DiagnosticSeverity, Position, PublishDiagnosticsParams, Url};

use crate::actions::EspxActionBuilder;

#[derive(Debug)]
pub enum EspxDiagnostic {
    ClearDiagnostics(Url),
    Publish(Vec<PublishDiagnosticsParams>),
}
impl TryInto<PublishDiagnosticsParams> for EspxActionBuilder {
    type Error = anyhow::Error;
    fn try_into(self) -> Result<PublishDiagnosticsParams, Self::Error> {
        match self {
            EspxActionBuilder::PromptOnLine { uri, line, .. } => {
                let range: lsp_types::Range = lsp_types::Range {
                    start: Position { line, character: 0 },
                    end: Position { line, character: 0 },
                };
                let message = String::from("Prompt code action available...");
                let diagnostic = Diagnostic {
                    range,
                    severity: Some(DiagnosticSeverity::HINT),
                    message,
                    ..Default::default()
                };
                Ok(PublishDiagnosticsParams {
                    uri,
                    diagnostics: vec![diagnostic],
                    version: None,
                })
            }
        }
    }
}

impl EspxDiagnostic {
    pub fn diagnose_document(text: &str, uri: Url) -> Self {
        let mut result_vec: Vec<PublishDiagnosticsParams> = vec![];
        if let Some(builders) = EspxActionBuilder::all_from_text_doc(&text, uri.clone()) {
            for builder in builders.into_iter() {
                if let Some(params) = builder.try_into().ok() {
                    result_vec.push(params);
                }
            }
        }
        if result_vec.is_empty() {
            return Self::ClearDiagnostics(uri);
        }
        Self::Publish(result_vec)
    }
}
