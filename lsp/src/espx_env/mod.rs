pub mod agents;
pub mod listeners;
use espionox::environment::{env_handle::EnvHandle, Environment};

use std::{collections::HashMap, sync::Arc};

use crate::{
    cache::GlobalCache,
    config::GLOBAL_CONFIG,
    espx_env::{agents::inner::InnerAgent, listeners::*},
};

#[derive(Debug)]
pub struct EspxEnv {
    environment: Environment,
    pub env_handle: EnvHandle,
}

impl EspxEnv {
    pub async fn init(cache: &GlobalCache) -> Self {
        let mut map = HashMap::new();
        let config = &GLOBAL_CONFIG;
        map.insert(
            config.model.provider.clone(),
            config.model.api_key.to_owned(),
        );

        let mut environment = Environment::new(None, map);

        let _ = agents::init_inner_agents(&mut environment).await;
        log::info!("Inner agents initialized");
        let _ = agents::init_indy_agents(&mut environment).await;
        log::info!("Indy agents initialized");

        let lru_rag = LRURAG::init(
            InnerAgent::Assistant.id(),
            Arc::clone(&cache.lru.listener_update),
        )
        .expect("Failed to build LRU RAG");
        environment.insert_listener(lru_rag).await.unwrap();

        let env_handle = environment.spawn_handle().unwrap();

        EspxEnv {
            environment,
            env_handle,
        }
    }
}
