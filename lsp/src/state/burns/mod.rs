mod activations;
pub mod error;
mod multiline;
mod singleline;
use super::{espx::AgentID, GlobalState};
use crate::{
    handle::{
        buffer_operations::{BufferOpChannelSender, BufferOperation},
        error::HandleResult,
    },
    parsing,
};
pub use activations::BurnActivation;
use anyhow::anyhow;
use espionox::agents::Agent;
use lsp_server::RequestId;
use lsp_types::{Position, Uri};
pub use multiline::MultiLineBurn;
pub use singleline::SingleLineBurn;
use tokio::sync::RwLockWriteGuard;
use tracing::warn;

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

#[tracing::instrument(name = "finding all burn activations in text")]
pub fn all_activations_in_text(text: &str) -> Vec<(Vec<u32>, BurnActivation)> {
    let mut all_burns = vec![];
    for burn in SingleLineBurn::all_variants() {
        let mut lines = parsing::all_lines_with_pattern(&burn.trigger_string(), &text);
        lines.append(&mut parsing::all_lines_with_pattern(
            &burn.echo_content(),
            &text,
        ));

        if !lines.is_empty() {
            warn!("burn variant {:?} found on lines: {:?}", burn, lines);
            all_burns.push((lines, BurnActivation::Single(burn)))
        }
    }

    for burn in MultiLineBurn::all_variants() {
        let lines = parsing::all_lines_with_pattern(&burn.trigger_string(), &text);
        if !lines.is_empty() {
            warn!("burn variant {:?} found on lines: {:?}", burn, lines);
            all_burns.push((lines, BurnActivation::Multi(burn)))
        }
    }

    all_burns
}
