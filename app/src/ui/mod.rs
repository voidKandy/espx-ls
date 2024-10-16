mod agents;
mod database;
mod documents;
mod home;
use crate::state::SharedState;
use agents::AgentsSectionState;
use database::DBSectionState;
use eframe::egui;
use egui::{Color32, Layout, RichText, Spinner, Ui};

pub fn run_gui(state: SharedState) -> eframe::Result {
    let options = eframe::NativeOptions {
        run_and_return: true,
        viewport: egui::ViewportBuilder::default().with_inner_size([1080.0, 720.0]),
        ..Default::default()
    };

    eframe::run_native(
        "ESPX - LS",
        options,
        Box::new(|_cc| Ok(Box::new(App::new(state)))),
    )
}

#[derive(Debug, Default, Clone, PartialEq, Eq, PartialOrd, Ord)]
pub enum UiSectionSelection {
    #[default]
    Home,
    Agents,
    Documents,
    Database,
}

impl AsRef<str> for UiSectionSelection {
    fn as_ref(&self) -> &str {
        match self {
            Self::Home => "Home",
            Self::Agents => "Agents",
            Self::Documents => "Documents",
            Self::Database => "Database",
        }
    }
}

impl UiSectionSelection {
    fn all_variants() -> Vec<Self> {
        vec![Self::Home, Self::Agents, Self::Documents, Self::Database]
    }

    fn render_fn(&self) -> Option<Box<dyn FnOnce(&mut Ui, &mut App)>> {
        match self {
            Self::Home => Some(Box::new(|ui, app| home::render_home_section(ui, app))),
            Self::Agents => Some(Box::new(|ui, app| agents::render_agents_section(ui, app))),
            Self::Documents => Some(Box::new(|ui, app| documents::render_docs_section(ui, app))),
            Self::Database => Some(Box::new(|ui, app| {
                database::render_database_section(ui, app)
            })),
        }
    }
}

struct App {
    state: SharedState,
    selected_section: UiSectionSelection,
    agents_section: AgentsSectionState,
    db_section: DBSectionState,
}

impl App {
    fn new(state: SharedState) -> Self {
        Self {
            state,
            selected_section: UiSectionSelection::default(),
            db_section: DBSectionState::default(),
            agents_section: AgentsSectionState::default(),
        }
    }
}

impl eframe::App for App {
    fn update(&mut self, ctx: &egui::Context, _frame: &mut eframe::Frame) {
        egui::SidePanel::left("Left Panel")
            .resizable(false)
            .show(ctx, |ui| {
                ui.with_layout(Layout::top_down(egui::Align::Min), |ui| {
                    for sect in UiSectionSelection::all_variants() {
                        let name = sect.as_ref().to_string();
                        ui.selectable_value(&mut self.selected_section, sect, name);
                    }
                });
            });

        egui::TopBottomPanel::top("Header").show(ctx, |ui| {
            let r = self.state.get_read().unwrap();
            ui.with_layout(Layout::right_to_left(egui::Align::Center), |ui| {
                egui::Frame::default()
                    .inner_margin(4.0)
                    .show(ui, |ui| match r.attached.as_ref() {
                        Some(add) => {
                            let richtext = RichText::new("✅").size(20.).color(Color32::GREEN);
                            ui.label(richtext)
                                .on_hover_text(format!("LSP attached at: {add:#?}"));
                        }
                        None => {
                            let spinner = Spinner::new().size(20.).color(Color32::ORANGE);
                            ui.add(spinner).on_hover_text("LSP is not attached");
                        }
                    });
            });
        });

        egui::CentralPanel::default().show(ctx, |ui| {
            if let Some(func) = self.selected_section.render_fn() {
                func(ui, self);
            }
        });
    }
}
