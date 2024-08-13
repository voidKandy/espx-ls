pub mod agents;
use crate::{config::GLOBAL_CONFIG, embeddings};
use anyhow::anyhow;
use espionox::{
    agents::{memory::OtherRoleTo, Agent},
    language_models::completions::streaming::ProviderStreamHandler,
    prelude::{stream_completion, AgentResult, ListenerTrigger, Message, MessageRole},
};
use std::{collections::HashMap, sync::LazyLock};
use tokio::sync::RwLockWriteGuard;
use tracing::debug;

use self::agents::{assistant_agent, sum_agent};

use super::{database::models::DBChunk, GlobalState};

#[derive(Debug)]
pub struct EspxEnv {
    // pub updater: AgentRagUpdater,
    pub agents: HashMap<AgentID, Agent>,
}

#[derive(Debug, Hash, Eq, PartialEq)]
pub enum AgentID {
    RAGAgent,
    QuickAgent,
    Summarizer,
}

static RAGROLE: LazyLock<MessageRole> = LazyLock::new(|| MessageRole::Other {
    alias: "RAG".to_owned(),
    coerce_to: OtherRoleTo::User,
});

impl EspxEnv {
    pub async fn init() -> anyhow::Result<Self> {
        let mut agents = HashMap::new();

        agents.insert(AgentID::QuickAgent, assistant_agent());
        agents.insert(AgentID::Summarizer, assistant_agent());

        Ok(EspxEnv { agents })
    }
}

#[tracing::instrument(name = "stream completion with RAG", skip_all)]
pub async fn stream_completion_with_rag(
    agent: &mut Agent,
    state_lock: &mut RwLockWriteGuard<'_, GlobalState>,
) -> AgentResult<ProviderStreamHandler> {
    let role = LazyLock::force(&RAGROLE);
    agent.cache.mut_filter_by(role, false);
    let last_user_message = agent
        .cache
        .pop(Some(MessageRole::User))
        .ok_or(anyhow!("no user message on stack"))?;

    let embedded_message = embeddings::get_passage_embeddings(vec![&last_user_message.content])?
        .pop()
        .unwrap();
    let db = state_lock.store.db.as_ref().ok_or(anyhow!("no database"))?;
    let relevant = DBChunk::get_relavent(&db.client, embedded_message, 0.5)
        .await
        .map_err(|err| anyhow!("problem getting relevant chunks: {:?}", err))?;
    debug!("got {} relevant chunks", relevant.len());
    for chunk in relevant {
        agent.cache.push(Message {
            role: role.clone(),
            content: chunk.to_string(),
        })
    }

    agent.cache.push(last_user_message);

    agent
        .do_action(stream_completion, (), Option::<ListenerTrigger>::None)
        .await
}
