use crate::{
    parsing,
    state::{burns::BurnActivation, store::GlobalStore},
};
use lsp_types::{Diagnostic, DiagnosticSeverity, Position, PublishDiagnosticsParams, Range, Uri};
use tracing::debug;

#[derive(Debug, Clone)]
pub enum LspDiagnostic {
    ClearDiagnostics(Uri),
    Publish(PublishDiagnosticsParams),
}

impl LspDiagnostic {
    #[tracing::instrument(name = "diagnosing document", skip(store))]
    pub fn diagnose_document(uri: Uri, store: &mut GlobalStore) -> anyhow::Result<Self> {
        let mut all_diagnostics = vec![];
        let text = store.get_doc(&uri)?;
        if let Some(burns) = store.burns.read_burns_on_doc(&uri) {
            for burn in burns.values() {
                let mut lines = parsing::all_lines_with_pattern(&burn.trigger_string(), &text);
                lines.append(&mut parsing::all_lines_with_pattern(
                    &burn.echo_content(),
                    &text,
                ));
                for l in lines {
                    let mut diags = Self::burn_diagnostics_on_line(&burn, l, &text)?;
                    all_diagnostics.append(&mut diags);
                }
            }
        }

        match all_diagnostics.is_empty() {
            false => {
                debug!("publishing diagnostics: {:?}", all_diagnostics);
                Ok(Self::Publish(PublishDiagnosticsParams {
                    uri,
                    diagnostics: all_diagnostics,
                    version: None,
                }))
            }
            true => {
                debug!("clearing diagnostics");
                Ok(Self::ClearDiagnostics(uri))
            }
        }
    }

    fn burn_diagnostics_on_line(
        burn: &BurnActivation,
        line_no: u32,
        text: &str,
    ) -> anyhow::Result<Vec<Diagnostic>> {
        let (userinput_info_opt, trigger_info) =
            burn.parse_for_user_input_and_trigger(line_no, text)?;
        let severity = Some(DiagnosticSeverity::HINT);

        let trigger_diagnostic = Diagnostic {
            range: Range {
                start: Position {
                    line: line_no as u32,
                    character: trigger_info.start as u32,
                },
                end: Position {
                    line: line_no as u32,
                    character: trigger_info.end as u32,
                },
            },
            severity,
            message: burn.trigger_diagnostic(),
            ..Default::default()
        };

        if let Some(userinput_info) = userinput_info_opt {
            let userinput_diagnostic = Diagnostic {
                range: Range {
                    start: Position {
                        line: line_no as u32,
                        character: userinput_info.start as u32,
                    },
                    end: Position {
                        line: line_no as u32,
                        character: userinput_info.end as u32,
                    },
                },
                severity,
                message: burn.user_input_diagnostic(),
                ..Default::default()
            };
            return Ok(vec![trigger_diagnostic, userinput_diagnostic]);
        }

        return Ok(vec![trigger_diagnostic]);
    }
}
