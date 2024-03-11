use anyhow::anyhow;
use espionox::{
    agents::{
        independent::IndependentAgent,
        language_models::{
            embed,
            openai::gpt::{Gpt, GptModel},
            LanguageModel,
        },
        memory::{embeddings::EmbeddingVector, MessageStack},
        Agent,
    },
    environment::{
        agent_handle::MessageRole,
        dispatch::{
            listeners::ListenerMethodReturn, Dispatch, EnvListener, EnvMessage, EnvRequest,
        },
        ListenerError,
    },
};

use crate::store::GLOBAL_STORE;

use super::ENVIRONMENT;

const SANITIZER_AGENT_SYSTEM_PROMPT: &str = r#"
You are a sanitizer agent. You will be given text to sanitize or adjust based on 
the needs of the user.
"#;

pub(super) fn sanitizer_agent() -> Agent {
    let gpt = LanguageModel::OpenAi(Gpt::new(GptModel::Gpt4, 0.4));
    Agent::new(SANITIZER_AGENT_SYSTEM_PROMPT, gpt)
}
