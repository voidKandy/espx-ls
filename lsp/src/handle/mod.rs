use lsp_server::Message;
use tracing_log::log::warn;

use self::{buffer_operations::BufferOpChannelHandler, error::HandleResult};

pub mod buffer_operations;
pub mod diagnostics;
pub mod error;
pub mod notifications;
pub mod requests;

pub fn handle_other(msg: Message) -> HandleResult<BufferOpChannelHandler> {
    warn!("unhandled message {:?}", msg);
    Ok(BufferOpChannelHandler::new())
}
pub type BufferOpChannelJoinHandle = tokio::task::JoinHandle<error::HandleResult<()>>;
