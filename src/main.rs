use std::sync::LazyLock;

use espx_lsp_server::start_lsp;
use tracing_log::log::info;
mod telemetry;

fn main() -> anyhow::Result<()> {
    LazyLock::force(&telemetry::TRACING);
    info!("Tracing Initialized");
    std::env::set_var("RUST_BACKTRACE", "1");
    start_lsp()?;
    Ok(())
}
