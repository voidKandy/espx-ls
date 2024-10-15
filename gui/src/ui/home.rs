use egui::Ui;

// pub fn setup_home_section(ui: &mut Ui, app: &mut super::App) {}

pub fn render_home_section(ui: &mut Ui, app: &mut super::App) {
    ui.label("Welcome");
    let r = app.state.get_read().unwrap();
    match r.attached.as_ref() {
        Some(add) => {
            ui.label(format!("the lsp is attached at {add:#?}"));
        }
        None => {
            ui.label("Lsp not attached");
        }
    }
}
