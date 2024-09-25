use crate::config::espx::{ModelConfig, ModelProvider};
use espionox::{
    agents::Agent,
    language_models::completions::{
        anthropic::builder::AnthropicCompletionModel, openai::builder::OpenAiCompletionModel,
        CompletionModel, CompletionProvider, ModelParameters,
    },
    prelude::Message,
};

mod sys_prompts;

pub(super) fn summarizer(cfg: ModelConfig) -> Agent {
    let provider: CompletionProvider = match cfg.provider {
        ModelProvider::OpenAi => OpenAiCompletionModel::Gpt3.into(),
        ModelProvider::Anthropic => AnthropicCompletionModel::Haiku.into(),
    };
    let params = ModelParameters::default();
    Agent::new(
        Some(self::sys_prompts::SUMMARIZER_AGENT_SYSTEM_PROMPT),
        CompletionModel::new(provider, params, &cfg.api_key),
    )
}

pub(super) fn global(cfg: ModelConfig) -> Agent {
    let provider: CompletionProvider = match cfg.provider {
        ModelProvider::OpenAi => OpenAiCompletionModel::Gpt4.into(),
        ModelProvider::Anthropic => AnthropicCompletionModel::Sonnet.into(),
    };
    let params = ModelParameters::default();
    Agent::new(
        Some(self::sys_prompts::ASSISTANT_AGENT_SYSTEM_PROMPT),
        CompletionModel::new(provider, params, &cfg.api_key),
    )
}

pub(super) fn document(cfg: ModelConfig, doc_content: &str) -> Agent {
    let provider: CompletionProvider = match cfg.provider {
        ModelProvider::OpenAi => OpenAiCompletionModel::Gpt4.into(),
        ModelProvider::Anthropic => AnthropicCompletionModel::Sonnet.into(),
    };
    let params = ModelParameters::default();
    let mut agent = Agent::new(
        Some(self::sys_prompts::ASSISTANT_AGENT_SYSTEM_PROMPT),
        CompletionModel::new(provider, params, &cfg.api_key),
    );

    agent.cache.push(Message::new_user(doc_content));

    agent
}
