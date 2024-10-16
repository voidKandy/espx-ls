mod agents;
mod config;
mod database;
mod embeddings;
mod error;
mod handle;
mod interact;
mod socket;
mod state;
mod telemetry;
mod ui;
mod util;
use self::{config::Config, state::SharedState};
use socket::{init_socket_listener_and_stream, unix_socket_loop};
use std::{
    io,
    sync::{Arc, LazyLock},
};
use tokio::sync::RwLock;
use tracing::warn;

#[tokio::main]
async fn main() -> eframe::Result<()> {
    LazyLock::force(&telemetry::TRACING);
    let config = Config::init();
    warn!("initializing with config: {config:#?}");
    let state = SharedState::init(config).await.unwrap();

    let unix_thread_state = state.clone();
    tokio::spawn(async move {
        let (unix_listener, unix_stream) = init_socket_listener_and_stream().await;
        let unix_stream = Arc::new(RwLock::new(unix_stream));
        unix_socket_loop(unix_stream, unix_listener, unix_thread_state).await
    });

    ui::run_gui(state)
}
