use crate::{
    embeddings,
    handle::buffer_operations::BufferOpChannelSender,
    state::database::{docs::chunks::DBDocumentChunk, Database},
};
use anyhow::anyhow;
use espionox::agents::{
    listeners::AgentListener,
    memory::{Message, MessageRole, MessageStack},
};
use std::{ops::DerefMut, sync::Arc};
use tokio::sync::{RwLock, RwLockReadGuard, RwLockWriteGuard};
use tracing::error;

struct ConvoUpdater {}

impl AgentListener for ConvoUpdater {
    fn trigger<'l>(&self) -> espionox::agents::listeners::ListenerTrigger {
        "updater".into()
    }
    fn sync_method<'l>(
        &'l mut self,
        _a: &'l mut espionox::agents::Agent,
    ) -> espionox::agents::error::AgentResult<()> {
        let convo_file = 

    }
}
