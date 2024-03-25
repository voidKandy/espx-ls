use espionox::{
    agents::independent::IndependentAgent,
    environment::{agent_handle::AgentHandle, Environment},
};
use once_cell::sync::Lazy;

use std::{
    collections::HashMap,
    sync::{Arc, Mutex},
};
pub mod independent;
pub mod inner;

use inner::*;

use self::independent::{all_indies, IndyAgent};

pub static INNER_AGENT_HANDLES: Lazy<Arc<Mutex<HashMap<InnerAgent, AgentHandle>>>> =
    Lazy::new(|| Arc::new(Mutex::new(HashMap::new())));
pub static INDY_AGENTS: Lazy<Arc<Mutex<HashMap<IndyAgent, IndependentAgent>>>> =
    Lazy::new(|| Arc::new(Mutex::new(HashMap::new())));

pub(super) async fn init_inner_agents(env: &mut Environment) {
    let agents = all_inner_agents();
    let mut handles = INNER_AGENT_HANDLES.lock().unwrap();
    for (var, a) in agents.into_iter() {
        let handle = env.insert_agent(Some(var.id()), a).await.unwrap();
        handles.insert(var, handle);
    }
}

pub(super) async fn init_indy_agents(env: &mut Environment) {
    let agents = all_indies();
    let mut handles = INDY_AGENTS.lock().unwrap();
    for (var, a) in agents.into_iter() {
        let a = env.make_agent_independent(a).await.unwrap();
        handles.insert(var, a);
    }
}

pub fn get_indy_agent(which: IndyAgent) -> Option<IndependentAgent> {
    let map = INDY_AGENTS.lock().unwrap();
    map.get(&which).cloned()
}

pub fn get_inner_agent_handle(which: InnerAgent) -> Option<AgentHandle> {
    let handles = INNER_AGENT_HANDLES.lock().unwrap();
    handles.get(&which).cloned()
}
