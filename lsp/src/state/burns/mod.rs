mod activations;
pub mod error;
mod multiline;
mod singleline;
use self::error::BurnResult;
use super::{espx::AgentID, GlobalState};
use crate::handle::{
    buffer_operations::{BufferOpChannelSender, BufferOperation},
    error::{HandleError, HandleResult},
};
pub use activations::BurnActivation;
use anyhow::anyhow;
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

    fn user_input_diagnostic(&self) -> Option<String>;
    fn trigger_diagnostic(&self) -> Option<String>;
    fn trigger_string(&self) -> String;
    fn doing_action_notification(&self) -> Option<BufferOperation>;
    async fn activate(
        &mut self,
        uri: Uri,
        request_id: Option<RequestId>,
        position: Option<Position>,
        sender: &mut BufferOpChannelSender,
        agent: &mut Agent,
        state_guard: &mut RwLockWriteGuard<'_, GlobalState>,
    ) -> HandleResult<()>;

    async fn activate_with_agent(
        &mut self,
        uri: Uri,
        request_id: Option<RequestId>,
        position: Option<Position>,
        sender: &mut BufferOpChannelSender,
        state_guard: &mut RwLockWriteGuard<'_, GlobalState>,
        agent_id: AgentID,
    ) -> HandleResult<()> {
        let mut agent = state_guard
            .espx_env
            .agents
            .remove(&agent_id)
            .ok_or(anyhow!("why no agent"))?;
        self.activate(uri, request_id, position, sender, &mut agent, state_guard)
            .await?;
        state_guard.espx_env.agents.insert(agent_id, agent);
        Ok(())
    }
}
