use std::{collections::HashSet, str::FromStr};

use eframe::egui;

use egui::{TextBuffer, Ui};
use egui_extras::{Column, TableBuilder};
use espionox::prelude::*;
use lsp_types::Uri;
use tracing::warn;

use crate::agents::Agents;

#[derive(Debug)]
pub struct EditingAgent {
    id: AgentID,
    system_prompt: String,
    completion_model: CompletionModel,
}

#[derive(Debug)]
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

#[derive(Debug)]
pub struct AgentsSectionState {
    // current_agent_name: String,
    should_get_editing: bool,
    editing_agent: Option<EditingAgent>,
}

impl Default for AgentsSectionState {
    fn default() -> Self {
        Self {
            // current_agent_name: "Global".to_string(),
            should_get_editing: true,
            editing_agent: None,
        }
    }
}

fn get_all_names(agents: &Agents) -> HashSet<String> {
    let mut all_names = HashSet::new();
    all_names.insert("Global".to_string());

    for ch in all_custom_names(agents).iter() {
        all_names.insert(ch.to_string());
    }
    for uri in all_doc_names(agents).iter() {
        all_names.insert(uri.to_string());
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

impl AgentsSectionState {
    pub fn update(&mut self, agents: &mut Agents) {
        let get_global = |a: &mut Agents| -> EditingAgent {
            EditingAgent::from_agent_and_id(a.global_agent_ref(), AgentID::Global)
        };
        if self.editing_agent.is_none() | self.should_get_editing {
            if let Some(current_agent_name) = self.current_agent_name() {
                warn!("current agent: {current_agent_name}");
                self.editing_agent = match current_agent_name.as_str() {
                    "Global" => Some(get_global(agents)),
                    _ if all_custom_names(agents).contains(&current_agent_name) => {
                        let char = current_agent_name.chars().next().unwrap();
                        agents
                            .custom_agent_ref(char)
                            .ok()
                            .and_then(|ag| Some(EditingAgent::from_agent_and_id(ag, char)))
                    }
                    _ if all_doc_names(agents).contains(&current_agent_name) => {
                        let uri = Uri::from_str(current_agent_name.as_str())
                            .expect("could not make uri from string");
                        agents
                            .doc_agent_ref(&uri)
                            .ok()
                            .and_then(|ag| Some(EditingAgent::from_agent_and_id(ag, uri)))
                    }
                    _ => None,
                };
            }

            self.should_get_editing = false;
            if self.editing_agent.is_none() {
                self.editing_agent = Some(get_global(agents));
            }
        }
    }

    fn current_agent_name(&self) -> Option<String> {
        match &self.editing_agent.as_ref()?.id {
            AgentID::Global => Some("Global".to_string()),
            AgentID::Char(s) | AgentID::Uri(s) => Some(s.to_string()),
        }
    }
}

pub fn render_agents_section(ui: &mut Ui, app: &mut super::App) {
    tracing::warn!("render agent section");
    let mut guard = app.state.get_write().unwrap();
    // app.agents_section.agents = guard.agents.as_mut();
    match guard.agents.as_mut() {
        Some(agents) => {
            app.agents_section.update(agents);
            let all_names = get_all_names(agents);
            // app.agents_section.current_agent_name = all_names[0].to_string();
            if let Some(ref mut current_agent_name) =
                &mut app.agents_section.current_agent_name().as_mut()
            {
                for name in all_names {
                    if ui
                        .add(egui::SelectableLabel::new(
                            **current_agent_name == name,
                            current_agent_name.to_string().as_str(),
                        ))
                        .clicked()
                    {
                        **current_agent_name = name;
                        app.agents_section.should_get_editing = true;
                    }
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
