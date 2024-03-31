pub mod agents;
mod listeners;
use espionox::environment::{env_handle::EnvHandle, Environment};

use std::{
    collections::HashMap,
    sync::{Arc, Mutex, OnceLock},
};

use crate::{
    cache::GLOBAL_CACHE,
    config::GLOBAL_CONFIG,
    espx_env::{agents::inner::InnerAgent, listeners::LRURAG},
};

pub static ENV: OnceLock<Arc<Mutex<Environment>>> = OnceLock::new();
pub static ENV_HANDLE: OnceLock<Arc<Mutex<EnvHandle>>> = OnceLock::new();

pub async fn init_espx_env() {
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

    let trigger = Arc::clone(&GLOBAL_CACHE.read().unwrap().lru.should_trigger_listener);
    let lru_rag =
        LRURAG::init(InnerAgent::Assistant.id(), trigger).expect("Failed to build LRU RAG");
    env.insert_listener(lru_rag).await.unwrap();

    let handle = env.spawn_handle().unwrap();

    let _ = ENV_HANDLE.set(Arc::new(Mutex::new(handle)));

    let _ = ENV.set(Arc::new(Mutex::new(env)));
}
