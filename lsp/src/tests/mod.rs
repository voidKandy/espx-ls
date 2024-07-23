mod database;
mod parsing;
mod store;
use once_cell::sync::Lazy;
use tracing::debug;

pub use crate::error::TRACING;

pub fn init_test_tracing() {
    Lazy::force(&TRACING);
    debug!("test tracing initialized");
}
