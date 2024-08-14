use super::{
    activations::{BurnActivation, BurnActivationVariant, BurnRange},
    error::BurnError,
};
use crate::{
    config::GLOBAL_CONFIG,
    handle::{
        buffer_operations::{BufferOpChannelSender, BufferOperation},
        error::HandleResult,
    },
    parsing,
};
use anyhow::anyhow;
use espionox::{agents::memory::OtherRoleTo, prelude::*};
use lsp_server::RequestId;
use lsp_types::{HoverContents, Position, Uri};
use serde::{Deserialize, Serialize};

#[derive(Clone, Serialize, Deserialize, Debug, PartialEq, Eq)]
pub enum MultiLineVariant {
    LockChunkIntoContext,
}

impl TryFrom<String> for MultiLineVariant {
    type Error = BurnError;
    fn try_from(value: String) -> Result<Self, Self::Error> {
        let str = value.as_str();
        let actions_config = &GLOBAL_CONFIG.user_actions;
        match str {
            _ if str == actions_config.lock_chunk_into_context => Ok(Self::LockChunkIntoContext),

            _ => Err(anyhow!("cannot create variant").into()),
        }
    }
}

#[derive(Clone, Serialize, Deserialize, Debug, PartialEq, Eq)]
pub struct MultiLineActivation {
    pub variant: MultiLineVariant,
    pub start_range: BurnRange,
    pub end_range: BurnRange,
}

impl BurnActivationVariant for MultiLineVariant {
    fn all() -> Vec<Self> {
        vec![Self::LockChunkIntoContext]
    }
}

impl BurnActivation<MultiLineVariant> for MultiLineActivation {
    fn doing_action_notification(&self) -> Option<BufferOperation> {
        None
    }

    fn trigger_pattern(&self) -> String {
        match self.variant {
            MultiLineVariant::LockChunkIntoContext => GLOBAL_CONFIG
                .user_actions
                .lock_chunk_into_context
                .to_owned(),
        }
        .to_string()
    }

    async fn activate(
        &mut self,
        uri: Uri,
        _request_id: Option<RequestId>,
        _position: Option<Position>,
        _sender: &mut BufferOpChannelSender,
        agent: &mut Agent,
        state_guard: &mut tokio::sync::RwLockWriteGuard<'_, crate::state::GlobalState>,
    ) -> HandleResult<Option<HoverContents>> {
        let doc = state_guard.store.get_doc(&uri)?;
        if let Some(inputs) = parsing::slices_between_pattern(&doc, &self.trigger_pattern()) {
            match self.variant {
                MultiLineVariant::LockChunkIntoContext => {
                    let role = MessageRole::Other {
                        alias: "LockChunkIntoContext".to_owned(),
                        coerce_to: OtherRoleTo::User,
                    };
                    agent.cache.mut_filter_by(&role, false);
                    agent.cache.push(Message {
                        role,
                        content: inputs
                            .iter()
                            .map(|i| i.text.as_str())
                            .collect::<Vec<&str>>()
                            .join(""),
                    });
                }
            }
        }

        Ok(None)
    }
}
