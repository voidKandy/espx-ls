use egui::{Layout, RichText, TextEdit, Ui};

use crate::state::SharedState;

use super::AppSectionState;

#[derive(Debug)]
pub struct DBSectionState {
    health_checking: bool,
}

impl Default for DBSectionState {
    fn default() -> Self {
        Self {
            health_checking: false,
        }
    }
}

impl AppSectionState for DBSectionState {
    fn render(&mut self, ui: &mut Ui, state: SharedState) {
        let r = state.get_read().unwrap();
        if let Some(db) = r.database.as_ref() {
            ui.with_layout(Layout::top_down_justified(egui::Align::Min), |ui| {
                let button = ui.button("Health Check");

                let namespace =
                    RichText::new(format!("Namespace: {} ", db.config.namespace)).size(20.);
                let database =
                    RichText::new(format!("Database: {} ", db.config.database)).size(20.);
                ui.label(namespace);
                ui.label(database);

                // if button.clicked() {
                //     let healthy = db.client.health().await;
                // }
            });
        }
    }
}
