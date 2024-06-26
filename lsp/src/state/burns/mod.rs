mod activations;
pub mod error;
mod multiline;
mod singleline;
use self::error::BurnResult;
use super::GlobalState;
use crate::handle::{
    buffer_operations::{BufferOpChannelSender, BufferOperation},
    error::{HandleError, HandleResult},
};
pub use activations::BurnActivation;
use espionox::agents::Agent;
use lsp_server::RequestId;
use lsp_types::{Position, Uri};
pub use multiline::MultiLineBurn;
pub use singleline::SingleLineBurn;
use tokio::sync::RwLockWriteGuard;

pub trait Burn {
    fn all_variants() -> Vec<Self>
    where
        Self: Sized;

    fn user_input_diagnostic(&self) -> String;
    fn trigger_string(&self) -> String;
    fn trigger_diagnostic(&self) -> String;
    fn doing_action_notification(&self) -> Option<BufferOperation>;
    async fn activate_on_document(
        &mut self,
        uri: Uri,
        request_id: Option<RequestId>,
        position: Option<Position>,
        sender: &mut BufferOpChannelSender,
        agent: &mut Agent,
        state_guard: &mut RwLockWriteGuard<'_, GlobalState>,
        // agent: &mut Agent,
    ) -> HandleResult<()>;
}
