pub mod agents;
pub mod listeners;
use espionox::environment::{env_handle::EnvHandle, Environment};

use std::{
    collections::HashMap,
    sync::{Arc, Mutex, OnceLock},
};

use crate::{
    config::GLOBAL_CONFIG,
    espx_env::{agents::inner::InnerAgent, listeners::*},
    state::SharedGlobalState,
};

pub static ENV: OnceLock<Arc<Mutex<Environment>>> = OnceLock::new();
pub static ENV_HANDLE: OnceLock<Arc<Mutex<EnvHandle>>> = OnceLock::new();

pub async fn init_espx_env(state: &SharedGlobalState) {
    let mut map = HashMap::new();
    let config = &GLOBAL_CONFIG;
    map.insert(
        config.model.provider.clone(),
        config.model.api_key.to_owned(),
    );

    let mut env = Environment::new(None, map);

    let _ = agents::init_inner_agents(&mut env).await;
    log::info!("Inner agents initialized");
    let _ = agents::init_indy_agents(&mut env).await;
    log::info!("Indy agents initialized");

    let lru_rag =
        LRURAG::init(InnerAgent::Assistant.id(), state.clone()).expect("Failed to build LRU RAG");

    // let conversation_updater =
    //     ConversationUpdate::init().expect("Failed to build conversation updater");

    env.insert_listener(lru_rag).await.unwrap();
    // env.insert_listener(conversation_updater).await.unwrap();

    let handle = env.spawn_handle().unwrap();

    let _ = ENV_HANDLE.set(Arc::new(Mutex::new(handle)));

    let _ = ENV.set(Arc::new(Mutex::new(env)));
}
