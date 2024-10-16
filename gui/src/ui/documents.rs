use egui::{Layout, TextEdit, Ui};
use lsp_types::Uri;

#[derive(Default)]
pub struct DocumentSectionState {
    current_document: Option<Uri>,
}

pub fn render_docs_section(ui: &mut Ui, app: &mut super::App) {
    let r = app.state.get_read().unwrap();
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
