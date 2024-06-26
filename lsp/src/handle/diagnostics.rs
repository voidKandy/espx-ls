use crate::{
    parsing,
    state::{
        burns::{Burn, BurnActivation, MultiLineBurn, SingleLineBurn},
        store::GlobalStore,
    },
};
use anyhow::Ok;
use lsp_types::{
    Diagnostic, DiagnosticSeverity, OneOf, Position, PublishDiagnosticsParams, Range, Uri,
};
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
            for burn in burns.values().cloned() {
                match burn.into_inner() {
                    lsp_types::OneOf::Left(single) => {
                        let mut lines =
                            parsing::all_lines_with_pattern(&single.trigger_string(), &text);
                        lines.append(&mut parsing::all_lines_with_pattern(
                            &single.echo_content(),
                            &text,
                        ));
                        for l in lines {
                            let mut diags =
                                Self::burn_diagnostics_on_line(OneOf::Left(&single), l, &text)?;
                            all_diagnostics.append(&mut diags);
                        }
                    }
                    lsp_types::OneOf::Right(multi) => {
                        let lines_and_chars = parsing::all_lines_with_pattern_with_char_positions(
                            &multi.trigger_string(),
                            &text,
                        );

                        for (l, _) in lines_and_chars {
                            let mut diags =
                                Self::burn_diagnostics_on_line(OneOf::Right(&multi), l, &text)?;
                            all_diagnostics.append(&mut diags);
                        }
                    }
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
        burn: OneOf<&SingleLineBurn, &MultiLineBurn>,
        line_no: u32,
        text: &str,
    ) -> anyhow::Result<Vec<Diagnostic>> {
        let severity = Some(DiagnosticSeverity::HINT);
        match burn {
            OneOf::Left(single) => {
                let (userinput_info_opt, trigger_info) =
                    single.parse_for_user_input_and_trigger(line_no, text)?;

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
                    message: single.trigger_diagnostic(),
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
                        message: single.user_input_diagnostic(),
                        ..Default::default()
                    };
                    return Ok(vec![trigger_diagnostic, userinput_diagnostic]);
                }

                return Ok(vec![trigger_diagnostic]);
            }
            OneOf::Right(multi) => {
                let (user_input_ranges, trigger_ranges) =
                    multi.parse_for_user_inputs_and_triggers(text)?;
                debug!("got ranges: {:?}{:?}", user_input_ranges, trigger_ranges);

                let mut all_diagnostics = trigger_ranges.into_iter().fold(vec![], |mut acc, tr| {
                    acc.push(Diagnostic {
                        range: tr.range,
                        severity,
                        message: multi.trigger_diagnostic(),
                        ..Default::default()
                    });
                    acc
                });

                user_input_ranges.into_iter().for_each(|uir| {
                    all_diagnostics.push(Diagnostic {
                        range: uir.range,
                        severity,
                        message: multi.user_input_diagnostic(),
                        ..Default::default()
                    });
                });
                return Ok(all_diagnostics);
            }
        }
    }
}
