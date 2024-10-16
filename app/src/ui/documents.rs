use egui::{Layout, TextEdit, Ui};
use lsp_types::Uri;

use crate::state::SharedState;

use super::AppSectionState;

#[derive(Debug, Default)]
pub struct DocumentSectionState {
    current_document: Option<Uri>,
}

impl AppSectionState for DocumentSectionState {
    fn render(&mut self, ui: &mut Ui, state: SharedState) {
        let r = state.get_read().unwrap();
        ui.with_layout(Layout::top_down(egui::Align::Min), |ui| {
            for (uri, doc) in r.documents.iter() {
                ui.label(uri.to_string());
                let mut val = doc.to_string().trim().to_owned();
                let textedit = TextEdit::multiline(&mut val)
                    .interactive(false)
                    .code_editor();
                ui.add(textedit);
            }
        });
    }
}
