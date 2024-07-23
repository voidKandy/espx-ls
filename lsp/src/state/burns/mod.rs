mod activations;
pub mod error;
mod multiline;
mod singleline;
use crate::{
    config::GLOBAL_CONFIG,
    handle::{buffer_operations::BufferOpChannelSender, error::HandleResult},
    parsing,
};

pub use self::{
    activations::{Activation, BurnActivation},
    multiline::{MultiLineActivation, MultiLineVariant},
    singleline::{SingleLineActivation, SingleLineVariant},
};
use activations::BurnRange;
use anyhow::anyhow;
use error::{BurnError, BurnResult};
use lsp_server::RequestId;
use lsp_types::{HoverContents, Position, Range, Uri};
use serde::{Deserialize, Serialize};
use serde_with::{serde_as, DisplayFromStr};
use tokio::sync::RwLockWriteGuard;
use tracing::{debug, warn};

use super::{espx::AgentID, GlobalState};

#[serde_as]
#[derive(Clone, Serialize, Deserialize, Debug, PartialEq, Eq)]
pub struct Burn {
    // https://github.com/surrealdb/surrealdb/issues/2233
    #[serde_as(as = "DisplayFromStr")]
    pub id: uuid::Uuid,
    pub activation: Activation,
    pub hover_contents: Option<HoverContents>,
}

impl From<SingleLineActivation> for Burn {
    fn from(value: SingleLineActivation) -> Self {
        Self {
            id: uuid::Uuid::new_v4(),
            activation: Activation::Single(value),
            hover_contents: None,
        }
    }
}

impl From<MultiLineActivation> for Burn {
    fn from(value: MultiLineActivation) -> Self {
        Self {
            id: uuid::Uuid::new_v4(),
            activation: Activation::Multi(value),
            hover_contents: None,
        }
    }
}

fn all_trigger_strings() -> Vec<String> {
    let a = GLOBAL_CONFIG.user_actions.clone();
    vec![
        a.quick_prompt.to_string(),
        a.rag_prompt.to_string(),
        a.walk_project.to_string(),
        a.lock_doc_into_context.to_string(),
        a.lock_doc_into_context.to_string(),
    ]
    .to_vec()
}

impl Burn {
    pub async fn activate_with_agent(
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

        self.hover_contents = match &mut self.activation {
            Activation::Multi(a) => {
                a.activate(uri, request_id, position, sender, &mut agent, state_guard)
                    .await?
            }
            Activation::Single(a) => {
                a.activate(uri, request_id, position, sender, &mut agent, state_guard)
                    .await?
            }
        };
        state_guard.espx_env.agents.insert(agent_id, agent);
        Ok(())
    }

    #[tracing::instrument(name = "finding all burn activations in text", skip_all)]
    pub fn all_in_text(text: &str) -> Vec<Burn> {
        let mut all_burns = vec![];
        for trigger in all_trigger_strings() {
            if let Some(slices) = parsing::slices_of_pattern(text, &trigger) {
                if let Some(variant) = SingleLineVariant::try_from(trigger.clone()).ok() {
                    for slice in slices {
                        all_burns.push(Burn::from(SingleLineActivation::new(
                            variant.clone(),
                            slice.range,
                        )));
                    }
                } else if let Some(variant) = MultiLineVariant::try_from(trigger.clone()).ok() {
                    if let Some(mut slices) = parsing::slices_of_pattern(text, &trigger) {
                        if slices.len() % 2 != 0 {
                            warn!("uneven amount of multiline burns, maybe one is unclosed?")
                        }
                        slices.reverse();
                        for _ in 0..slices.len() / 2 {
                            if let Some(start_range) =
                                slices.pop().and_then(|s| Some(BurnRange::from(s.range)))
                            {
                                if let Some(end_range) =
                                    slices.pop().and_then(|s| Some(BurnRange::from(s.range)))
                                {
                                    all_burns.push(Burn::from(MultiLineActivation {
                                        variant: variant.clone(),
                                        start_range,
                                        end_range,
                                    }));
                                }
                            }
                        }
                    }
                }
            }
        }
        debug!("returning burns: {:?}", all_burns);

        all_burns
    }

    pub fn update_activation(&mut self, other: Self) -> BurnResult<()> {
        if !self.activation.matches_variant(&other.activation) {
            return Err(BurnError::WrongVariant);
        }

        match &mut self.activation {
            Activation::Single(ref mut a) => {
                a.range = other.activation.range().take_left().unwrap().to_owned()
            }
            Activation::Multi(ref mut a) => {
                (a.start_range, a.end_range) = other
                    .activation
                    .range()
                    .take_right()
                    .and_then(|(s, e)| Some((s.to_owned(), e.to_owned())))
                    .unwrap()
                    .to_owned()
            }
        }

        Ok(())
    }
}
