pub mod agents;
pub mod error;
pub mod listeners;
use espionox::environment::{env_handle::EnvHandle, Environment};

use std::{collections::HashMap, sync::Arc};

use crate::{
    config::GLOBAL_CONFIG,
    espx_env::{agents::inner::InnerAgent, error::EspxEnvError, listeners::*},
    store::GlobalStore,
};

use self::error::EspxEnvResult;

#[derive(Debug)]
pub struct EspxEnv {
    environment: Environment,
    pub env_handle: EnvHandle,
}

impl EspxEnv {
    pub async fn init(store: &GlobalStore) -> EspxEnvResult<Self> {
        let mut map = HashMap::new();
        match &GLOBAL_CONFIG.model {
            Some(config) => {
                map.insert(config.provider.clone(), config.api_key.to_owned());
            }
            None => return Err(EspxEnvError::NoConfig),
        }

        let mut environment = Environment::new(None, map);

        let _ = agents::init_inner_agents(&mut environment).await;
        log::info!("Inner agents initialized");
        let _ = agents::init_indy_agents(&mut environment).await;
        log::info!("Indy agents initialized");

        let lru_rag =
            LRURAG::init(store.updater.quick.clone_message()).expect("Failed to build LRU RAG");
        environment.insert_listener(lru_rag).await?;

        let db_rag =
            DBRAG::init(store.updater.db.clone_message()).expect("Failed to build LRU RAG");
        environment.insert_listener(db_rag).await?;

        let env_handle = environment.spawn_handle()?;

        Ok(EspxEnv {
            environment,
            env_handle,
        })
    }
}
