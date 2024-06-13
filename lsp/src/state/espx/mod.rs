pub mod agents;
pub mod listeners;
use crate::config::GLOBAL_CONFIG;
use espionox::agents::Agent;
use listeners::{AssistantUpdater, RefCountedUpdater};
use std::collections::HashMap;

use self::agents::{assistant_agent, sum_agent};

#[derive(Debug)]
pub struct EspxEnv {
    pub updater: RefCountedUpdater,
    pub agents: HashMap<AgentID, Agent>,
}

#[derive(Debug, Hash, Eq, PartialEq)]
pub enum AgentID {
    Assistant,
    Summarizer,
}

impl EspxEnv {
    pub async fn init() -> anyhow::Result<Self> {
        let mut agents = HashMap::new();

        let mut ass = assistant_agent();
        let assistant_updater: RefCountedUpdater =
            AssistantUpdater::init(GLOBAL_CONFIG.database.is_some()).into();
        ass.insert_listener(assistant_updater.clone());

        agents.insert(AgentID::Assistant, ass);
        agents.insert(AgentID::Summarizer, sum_agent());

        Ok(EspxEnv {
            updater: assistant_updater,
            agents,
        })
    }
}
