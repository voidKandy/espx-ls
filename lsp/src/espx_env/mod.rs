pub mod agents;
pub mod error;
pub mod listeners;
use self::error::EspxEnvResult;
use crate::{
    config::GLOBAL_CONFIG,
    espx_env::{agents::inner::InnerAgent, error::EspxEnvError, listeners::*},
    store::GlobalStore,
};
use espionox::environment::{env_handle::EnvHandle, Environment};
use listeners::{AssistantUpdater, RefCountedUpdater};
use std::{collections::HashMap, sync::Arc};

#[derive(Debug)]
pub struct EspxEnv {
    environment: Environment,
    pub updater: RefCountedUpdater,
    pub env_handle: EnvHandle,
}

impl EspxEnv {
    pub async fn init() -> EspxEnvResult<Self> {
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

        let assistant_updater: RefCountedUpdater =
            AssistantUpdater::init(GLOBAL_CONFIG.database.is_some())?.into();
        environment
            .insert_listener(assistant_updater.clone())
            .await?;

        let env_handle = environment.spawn_handle()?;

        Ok(EspxEnv {
            environment,
            updater: assistant_updater,
            env_handle,
        })
    }
}
