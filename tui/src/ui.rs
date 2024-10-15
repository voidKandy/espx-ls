#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")] // hide console window on Windows in release
#![allow(rustdoc::missing_crate_level_docs)] // it's an example

use std::str::FromStr;

use eframe::egui;
use egui::{Layout, TextBuffer, Ui};
use espionox::agents::Agent;
use lsp_types::Uri;
use tokio::sync::RwLockWriteGuard;

use crate::{
    agents::Agents,
    state::{EnvironmentState, SharedState},
};

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
    Logs,
}

impl AsRef<str> for UiSectionSelection {
    fn as_ref(&self) -> &str {
        match self {
            Self::Home => "Home",
            Self::Logs => "Logs",
            Self::Agents => "Agents",
            Self::Documents => "Documents",
            Self::Database => "Database",
        }
    }
}

impl UiSectionSelection {
    fn all_variants() -> Vec<Self> {
        vec![
            Self::Home,
            Self::Logs,
            Self::Agents,
            Self::Documents,
            Self::Database,
        ]
    }

    fn render_fn(&self) -> Option<Box<dyn FnOnce(&mut Ui, &mut App)>> {
        match self {
            Self::Home => None,
            Self::Agents => Some(Box::new(|ui, app| render_agents_section(ui, app))),
            Self::Logs => None,
            Self::Database => None,
            Self::Documents => None,
        }
    }
}

struct App {
    state: SharedState,
    selected_section: UiSectionSelection,
    agents_section: AgentsSection,
}

impl App {
    fn new(state: SharedState) -> Self {
        Self {
            state,
            selected_section: UiSectionSelection::default(),
            agents_section: AgentsSection {
                current_agent_name: "".to_string(),
            },
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
            // ui.label("panel");
            // table(ui);

            // if ui.button("Close").clicked() {
            //     ui.ctx().send_viewport_cmd(egui::ViewportCommand::Close);
            // }
        });

        egui::CentralPanel::default().show(ctx, |ui| {
            ui.label(self.selected_section.as_ref());
            if let Some(func) = self.selected_section.render_fn() {
                func(ui, self);
            }
        });
    }
}

pub struct AgentsSection {
    current_agent_name: String,
    // agents: Option<&'a mut Agents>,
}

fn render_agents_section(ui: &mut Ui, app: &mut App) {
    let mut guard = app.state.get_write().unwrap();
    // app.agents_section.agents = guard.agents.as_mut();
    match guard.agents.as_mut() {
        Some(agents) => {
            let mut all_names = vec!["Global".to_string()];
            app.agents_section.current_agent_name = all_names[0].to_string();

            let current_agent_name = &mut app.agents_section.current_agent_name;
            let all_custom_names = agents
                .custom_agents_iter()
                .map(|(n, _)| n.to_string())
                .collect::<Vec<String>>();
            let all_doc_names = agents
                .doc_agents_iter()
                .map(|(n, _)| n.to_string())
                .collect::<Vec<String>>();

            for ch in all_custom_names.iter() {
                all_names.push(ch.to_string());
            }
            for uri in all_doc_names.iter() {
                all_names.push(uri.to_string());
            }

            for name in all_names {
                if ui
                    .add(egui::SelectableLabel::new(
                        *current_agent_name == name,
                        name.as_str(),
                    ))
                    .clicked()
                {
                    *current_agent_name = name;
                }
            }

            if let Some(agent) = match current_agent_name.as_str() {
                "Global" => Some(agents.global_agent_mut()),
                _ if all_custom_names.contains(current_agent_name) => agents
                    .custom_agent_mut(current_agent_name.chars().next().unwrap())
                    .ok(),
                _ if all_doc_names.contains(current_agent_name) => {
                    let uri =
                        Uri::from_str(current_agent_name).expect("could not make uri from string");
                    agents.doc_agent_mut(&uri).ok()
                }
                _ => None,
            } {
                ui.label(format!("Completion Model: {:#?}", agent.completion_model));
                ui.label(format!("MessageStack: {:#?}", agent.cache));
            }
        }
        None => {
            ui.label("No Agents");
        }
    }
}
