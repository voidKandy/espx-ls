use std::str::FromStr;

use eframe::egui;
use egui::{TextBuffer, Ui};
use egui_extras::{Column, TableBuilder};
use espionox::prelude::*;
use lsp_types::Uri;

use crate::agents::Agents;

pub struct EditingAgent {
    id: AgentID,
    system_prompt: String,
    completion_model: CompletionModel,
}

enum AgentID {
    Global,
    Uri(String),
    Char(String),
}

impl Into<AgentID> for char {
    fn into(self) -> AgentID {
        AgentID::Char(self.to_string())
    }
}

impl Into<AgentID> for Uri {
    fn into(self) -> AgentID {
        AgentID::Uri(self.to_string())
    }
}

impl EditingAgent {
    fn from_agent_and_id(agent: &Agent, id: impl Into<AgentID>) -> Self {
        let system_prompt = agent
            .cache
            .clone()
            .into_iter()
            .nth(0)
            .and_then(|m| {
                if m.role == MessageRole::System {
                    Some(m.content)
                } else {
                    None
                }
            })
            .unwrap_or(String::new())
            .trim()
            .to_string();

        Self {
            id: id.into(),
            system_prompt,
            completion_model: agent.completion_model.clone(),
        }
    }
}

pub struct AgentsSectionState {
    current_agent_name: String,
    editing_agent: Option<EditingAgent>,
}

impl Default for AgentsSectionState {
    fn default() -> Self {
        Self {
            current_agent_name: "Global".to_string(),
            editing_agent: None,
        }
    }
}

fn get_all_names(agents: &Agents) -> Vec<String> {
    let mut all_names = vec!["Global".to_string()];

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
    all_names
}

fn all_custom_names(agents: &Agents) -> Vec<String> {
    agents
        .custom_agents_iter()
        .map(|(n, _)| n.to_string())
        .collect::<Vec<String>>()
}
fn all_doc_names(agents: &Agents) -> Vec<String> {
    agents
        .doc_agents_iter()
        .map(|(n, _)| n.to_string())
        .collect::<Vec<String>>()
}

pub fn setup_agents_section(ui: &mut Ui, app: &mut super::App) {
    let mut guard = app.state.get_write().unwrap();
    // app.agents_section.agents = guard.agents.as_mut();
    match guard.agents.as_mut() {
        Some(agents) => {
            if app.agents_section.editing_agent.is_none() {
                app.agents_section.editing_agent =
                    match app.agents_section.current_agent_name.as_str() {
                        "Global" => Some(EditingAgent::from_agent_and_id(
                            agents.global_agent_ref(),
                            AgentID::Global,
                        )),
                        _ if all_custom_names(agents)
                            .contains(&app.agents_section.current_agent_name) =>
                        {
                            let char = app
                                .agents_section
                                .current_agent_name
                                .chars()
                                .next()
                                .unwrap();
                            agents
                                .custom_agent_ref(char)
                                .ok()
                                .and_then(|ag| Some(EditingAgent::from_agent_and_id(ag, char)))
                        }
                        _ if all_doc_names(agents)
                            .contains(&app.agents_section.current_agent_name) =>
                        {
                            let uri = Uri::from_str(app.agents_section.current_agent_name.as_str())
                                .expect("could not make uri from string");
                            agents
                                .doc_agent_ref(&uri)
                                .ok()
                                .and_then(|ag| Some(EditingAgent::from_agent_and_id(ag, uri)))
                        }
                        _ => None,
                    };
            }
        }
        None => {
            ui.label("No Agents");
        }
    }
}

pub fn render_agents_section(ui: &mut Ui, app: &mut super::App) {
    let mut guard = app.state.get_write().unwrap();
    // app.agents_section.agents = guard.agents.as_mut();
    match guard.agents.as_mut() {
        Some(agents) => {
            let mut all_names = get_all_names(agents);
            app.agents_section.current_agent_name = all_names[0].to_string();
            let current_agent_name = &mut app.agents_section.current_agent_name;

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

            if let Some(editing) = app.agents_section.editing_agent.as_mut() {
                ui.label("System Prompt");
                ui.text_edit_multiline(&mut editing.system_prompt);
            }
        }
        None => {
            ui.label("No Agents");
        }
    }
}
