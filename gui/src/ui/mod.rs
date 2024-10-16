mod agents;
mod documents;
mod home;
use crate::state::SharedState;
use agents::AgentsSectionState;
use eframe::egui;
use egui::{Layout, RichText, Ui};

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
            Self::Database => None,
        }
    }
}

struct App {
    state: SharedState,
    selected_section: UiSectionSelection,
    agents_section: AgentsSectionState,
}

impl App {
    fn new(state: SharedState) -> Self {
        Self {
            state,
            selected_section: UiSectionSelection::default(),
            agents_section: AgentsSectionState::default(),
        }
    }
}

impl eframe::App for App {
    fn update(&mut self, ctx: &egui::Context, _frame: &mut eframe::Frame) {
        egui::TopBottomPanel::top("Header").show(ctx, |ui| {
            ui.with_layout(Layout::left_to_right(egui::Align::Min), |ui| {
                for sect in UiSectionSelection::all_variants() {
                    let name = sect.as_ref().to_string();
                    ui.selectable_value(&mut self.selected_section, sect, name);
                }
            });
        });

        egui::CentralPanel::default().show(ctx, |ui| {
            ui.vertical_centered(|ui| {
                let richtext = RichText::new(self.selected_section.as_ref()).size(30.0);
                ui.label(richtext);
                ui.separator();
            });
            if let Some(func) = self.selected_section.render_fn() {
                func(ui, self);
            }
        });
    }
}
