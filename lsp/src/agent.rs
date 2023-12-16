use espionox::{agents::Agent, memory::Memory};
use std::{
    collections::HashMap,
    sync::{Arc, Mutex, OnceLock},
    thread,
};
use tokio::runtime::Runtime;
use tokio::time::*;

pub static AGENT: OnceLock<Arc<Mutex<Agent>>> = OnceLock::new();
pub fn init_agent() {
    _ = AGENT.set(Arc::new(Mutex::new(Agent::default())));
    log::warn!("AGENT INITIALIZED");
}

pub fn block_prompt(prompt: &str) -> String {
    let mut a = AGENT
        .get()
        .expect("Can't get static agent")
        .lock()
        .expect("Can't lock static agent");
    let rt = Runtime::new().unwrap();
    log::info!("PROMPTING AGENT");
    rt.block_on(async move {
        let p = a
            .prompt(prompt.to_string())
            .await
            .expect("Failed to get completion");
        log::info!("AGENT REPONDED: {}", p);
        p
    })
}
