use espx_lsp_server::start_lsp;
use once_cell::sync::Lazy;
use tracing_log::log::info;
mod telemetry;

fn main() -> anyhow::Result<()> {
    Lazy::force(&telemetry::TRACING);
    info!("Tracing Initialized");
    start_lsp()?;
    Ok(())
}
