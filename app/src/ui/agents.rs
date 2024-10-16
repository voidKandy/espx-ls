use crate::agents::{error::AgentsResult, Agents};
use anyhow::anyhow;
use eframe::egui;
use egui::{Color32, ScrollArea, SelectableLabel, TextEdit, Ui};
use espionox::prelude::*;
use lsp_types::Uri;
use std::{collections::HashSet, hash::Hash, str::FromStr};
use tracing::warn;

#[derive(Debug)]
pub struct EditingAgent {
    id: UiAgentID,
    system_prompt: String,
    all_other_messages: MessageStack,
    completion_model: CompletionModel,
}

#[derive(Debug, Clone, Hash, PartialEq, Eq)]
enum UiAgentID {
    Global,
    Uri(String),
    Char(String),
}

#[derive(Debug)]
pub struct AgentsSectionState {
    all_names: HashSet<UiAgentID>,
    should_switch_to_agent: Option<UiAgentID>,
    editing_agent: Option<EditingAgent>,
    try_update_agent: bool,
}

const GLOBAL_AGENT_NAME: &str = "Global";

impl UiAgentID {
    fn ui_display(&self) -> String {
        match self {
            Self::Global => GLOBAL_AGENT_NAME.to_string(),
            Self::Uri(uri) => {
                let split = uri
                    .rsplitn(3, std::path::MAIN_SEPARATOR)
                    .collect::<Vec<&str>>();
                format!("{}{}{}", split[1], std::path::MAIN_SEPARATOR, split[0])
            }
            Self::Char(ch) => {
                format!("Custom Agent({ch})")
            }
        }
    }
}

impl TryInto<Uri> for UiAgentID {
    type Error = anyhow::Error;
    fn try_into(self) -> Result<Uri, Self::Error> {
        if let Self::Uri(uri_str) = self {
            return Ok(Uri::from_str(&uri_str).expect("could not create URI"));
        }
        Err(anyhow!("Incorrect variant"))
    }
}

impl TryInto<char> for UiAgentID {
    type Error = anyhow::Error;
    fn try_into(self) -> Result<char, Self::Error> {
        if let Self::Char(char_str) = self {
            return Ok(char_str.chars().nth(0).unwrap());
        }
        Err(anyhow!("Incorrect variant"))
    }
}

impl Into<UiAgentID> for char {
    fn into(self) -> UiAgentID {
        UiAgentID::Char(self.to_string())
    }
}

impl Into<UiAgentID> for Uri {
    fn into(self) -> UiAgentID {
        UiAgentID::Uri(self.to_string())
    }
}

impl EditingAgent {
    fn from_agent_and_id(agent: &Agent, id: impl Into<UiAgentID>) -> Self {
        let system_prompt = agent
            .cache
            .ref_system_prompt_content()
            .unwrap_or("")
            .trim()
            .to_string();

        let all_other_messages: MessageStack = agent
            .cache
            .ref_filter_by(&MessageRole::System, false)
            .into();

        Self {
            id: id.into(),
            system_prompt,
            all_other_messages,
            completion_model: agent.completion_model.clone(),
        }
    }

    fn mut_agent_ref<'agents>(
        &self,
        agents: &'agents mut Agents,
    ) -> AgentsResult<&'agents mut Agent> {
        match &self.id {
            UiAgentID::Uri(uri_str) => {
                let uri = Uri::from_str(&uri_str).expect("could not get uri from uri str");
                agents.doc_agent_mut(&uri)
            }
            UiAgentID::Char(char) => {
                let char = char.chars().nth(0).expect("empty char str");
                agents.custom_agent_mut(char)
            }
            UiAgentID::Global => Ok(agents.global_agent_mut()),
        }
    }
    /// Assumes self is more up to date than agent, if they are out of sync, updates agent to the
    /// state of self
    fn try_sync_with_agent(&self, agent: &mut Agent) {
        if let Some(system_prompt) = agent.cache.mut_system_prompt_content() {
            if self.system_prompt != *system_prompt {
                *system_prompt = self.system_prompt.clone();
            }
        }
        if agent.completion_model != self.completion_model {
            agent.completion_model = self.completion_model.clone();
        }
    }
}

impl Default for AgentsSectionState {
    fn default() -> Self {
        Self {
            all_names: HashSet::new(),
            should_switch_to_agent: Some(UiAgentID::Global),
            editing_agent: None,
            try_update_agent: false,
        }
    }
}

fn get_all_names(agents: &Agents) -> HashSet<UiAgentID> {
    let mut all_names = HashSet::new();
    all_names.insert(UiAgentID::Global);

    for id in all_custom_names(agents) {
        all_names.insert(id);
    }

    for id in all_doc_names(agents) {
        all_names.insert(id);
    }

    all_names
}

fn all_custom_names(agents: &Agents) -> Vec<UiAgentID> {
    agents
        .custom_agents_iter()
        .map(|(n, _)| (*n).into())
        .collect::<Vec<UiAgentID>>()
}
fn all_doc_names(agents: &Agents) -> Vec<UiAgentID> {
    agents
        .doc_agents_iter()
        .map(|(n, _)| n.clone().into())
        .collect::<Vec<UiAgentID>>()
}

impl AgentsSectionState {
    pub fn update(&mut self, agents: &mut Agents) {
        let get_global = |a: &mut Agents| -> EditingAgent {
            EditingAgent::from_agent_and_id(a.global_agent_ref(), UiAgentID::Global)
        };

        if self.try_update_agent {
            if let Some(editing_agent) = self.editing_agent.as_ref() {
                if let Ok(agent) = editing_agent.mut_agent_ref(agents) {
                    warn!("trying to update agent {agent:#?}");
                    editing_agent.try_sync_with_agent(agent);
                    self.try_update_agent = false;
                }
            }
        }

        let all_names = get_all_names(agents);
        if all_names != self.all_names {
            warn!("updating all names: {all_names:#?}");
            self.all_names = all_names
        }

        if let Some(switch_to_agent) = self.should_switch_to_agent.take() {
            self.editing_agent = match switch_to_agent {
                UiAgentID::Global => Some(get_global(agents)),
                UiAgentID::Char(_) => {
                    let char: char = switch_to_agent.try_into().expect("failed to get char");
                    agents
                        .custom_agent_ref(char)
                        .ok()
                        .and_then(|ag| Some(EditingAgent::from_agent_and_id(ag, char)))
                }
                UiAgentID::Uri(_) => {
                    let uri: Uri = switch_to_agent.try_into().expect("failed to get uri");
                    agents
                        .doc_agent_ref(&uri)
                        .ok()
                        .and_then(|ag| Some(EditingAgent::from_agent_and_id(ag, uri)))
                }
            };

            if self.editing_agent.is_none() {
                self.editing_agent = Some(get_global(agents));
            }
        }
    }

    fn current_agent_id(&self) -> Option<&UiAgentID> {
        Some(&self.editing_agent.as_ref()?.id)
    }
}

pub fn render_agents_section(ui: &mut Ui, app: &mut super::App) {
    tracing::warn!("render agent section");
    let mut guard = app.state.get_write().unwrap();
    match guard.agents.as_mut() {
        Some(agents) => {
            app.agents_section.update(agents);

            let selectable_labels =
                |current_name: &UiAgentID| -> Vec<(SelectableLabel, &UiAgentID)> {
                    app.agents_section
                        .all_names
                        .iter()
                        .map(|n| {
                            (
                                egui::SelectableLabel::new(current_name == n, n.ui_display()),
                                n,
                            )
                        })
                        .collect()
                };

            if let Some(current_agent_name) = app.agents_section.current_agent_id() {
                for (label, name) in selectable_labels(&current_agent_name) {
                    ui.horizontal_top(|ui| {
                        if ui.add(label).clicked() {
                            warn!("clicked label. Changing current name to {name:#?}");
                            app.agents_section.should_switch_to_agent = Some(name.to_owned());
                        }
                    });
                }
            }

            if let Some(editing) = app.agents_section.editing_agent.as_mut() {
                ui.vertical_centered_justified(|ui| {
                    ScrollArea::vertical().show(ui, |ui| {
                        ui.label("System Prompt");
                        let textedit = TextEdit::multiline(&mut editing.system_prompt)
                            .interactive(true)
                            .min_size(egui::Vec2 { x: 25., y: 1. });
                        if ui.add(textedit).changed() {
                            app.agents_section.try_update_agent = true;
                        }

                        for message in editing.all_other_messages.as_ref().iter() {
                            ui.label(message.role.to_string().to_uppercase());
                            let mut content = message.content.clone();
                            let singleline = {
                                if content.lines().count() < 2 {
                                    true
                                } else if content.lines().count() < 3
                                    && content
                                        .lines()
                                        .into_iter()
                                        .nth(2)
                                        .is_some_and(|l| l.trim().is_empty())
                                {
                                    true
                                } else {
                                    false
                                }
                            };
                            let color = match message.role.actual() {
                                MessageRole::User => Color32::from_rgb(255, 223, 223),
                                MessageRole::Assistant => Color32::from_rgb(210, 220, 255),
                                _ => Color32::from_rgb(255, 224, 230),
                            };

                            let textedit = match singleline {
                                true => TextEdit::singleline(&mut content)
                                    .interactive(false)
                                    .code_editor()
                                    .frame(false)
                                    .text_color(color),
                                false => TextEdit::multiline(&mut content)
                                    .interactive(false)
                                    .code_editor()
                                    .frame(false)
                                    .text_color(color)
                                    .min_size(egui::Vec2 { x: 25., y: 1. }),
                            };
                            ui.add(textedit);
                        }
                    });
                });
            }
        }
        None => {
            ui.label("No Agents");
        }
    }
}
