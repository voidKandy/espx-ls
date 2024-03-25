use espionox::language_models::ModelProvider;
use serde::{Deserialize, Serialize};
use std::{fs, path::Path};
use toml;

#[derive(Debug, Deserialize, Serialize)]
pub struct Config {
    model: ModelConfig,
}

#[derive(Debug, Deserialize, Serialize)]
pub struct ModelConfig {
    provider: ModelProvider,
    api_key: String,
}

pub fn echo_markerfile() {
    let path = Path::new("espx-ls.toml");

    let content = fs::read_to_string(path).unwrap();
    log::info!("CONFIG FILE: {:?}", content);
    let config: Config = match toml::from_str(&content) {
        Ok(c) => c,
        Err(err) => panic!("CONFIG ERROR: {:?}", err),
    };
    log::info!("CONFIG: {:?}", config);
}
