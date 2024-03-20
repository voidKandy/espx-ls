pub mod agents;
mod listeners;
use espionox::{
    environment::{env_handle::EnvHandle, Environment},
    language_models::ModelProvider,
};

use std::{
    collections::HashMap,
    env,
    sync::{Arc, Mutex, OnceLock},
};

pub static ENV: OnceLock<Arc<Mutex<Environment>>> = OnceLock::new();
pub static ENV_HANDLE: OnceLock<Arc<Mutex<EnvHandle>>> = OnceLock::new();

pub async fn init_statics() {
    dotenv::dotenv().ok();
    let api_key = env::var("OPENAI_API_KEY").expect("Could not get api key");
    let mut map = HashMap::new();
    map.insert(ModelProvider::OpenAi, api_key.to_owned());
    log::warn!("API KEY!: {:?}", api_key);

    let mut env = Environment::new(None, map);

    let _ = agents::init_inner_agents(&mut env).await;
    log::info!("Inner agents initialized");
    let _ = agents::init_indy_agents(&mut env).await;
    log::info!("Indy agents initialized");

    let handle = env.spawn_handle().unwrap();

    let _ = ENV_HANDLE.set(Arc::new(Mutex::new(handle)));

    let _ = ENV.set(Arc::new(Mutex::new(env)));
}
