use espionox::{
    agents::Agent,
    language_models::completions::{
        anthropic::builder::AnthropicCompletionModel, openai::builder::OpenAiCompletionModel,
        CompletionModel, CompletionProvider, ModelParameters,
    },
};

use crate::config::{ModelProvider, GLOBAL_CONFIG};

const ASSISTANT_AGENT_SYSTEM_PROMPT: &str = r#"
You are an AI assistant in NeoVim. You will be provided with the user's codebase, as well as their most recent changes to the current file
answer their queries to the best of your ability. Your response should consider the language of the user's codebase and current document.
"#;

const SUMMARIZER_AGENT_SYSTEM_PROMPT: &str = r#"
    You are a state of the art high quality code summary generator. 
    You will be provided with chunks of code that you must summarize.
    Please be thorough in your summaries.
"#;

pub const SUMMARIZE_WHOLE_DOC_PROMPT: &str = r#"
    Summarize this document to the best of your ability. Your summary should
    provide enough information to give the reader a good understanding of what
    the function of most, if not all, of the code in the document. 
"#;

pub const SUMMARIZE_DOC_CHUNK_PROMPT: &str = r#"
    Summarize the given document chunk. Your summary should
    provide enough information to give the reader a good understanding of what
    the function the code in the chunk. 
"#;

pub(super) fn sum_agent() -> Agent {
    let cfg = &GLOBAL_CONFIG.model.as_ref().expect("No config");
    let provider: CompletionProvider = match cfg.provider {
        ModelProvider::OpenAi => OpenAiCompletionModel::Gpt3.into(),
        ModelProvider::Anthropic => AnthropicCompletionModel::Haiku.into(),
    };
    let params = ModelParameters::default();
    Agent::new(
        Some(SUMMARIZER_AGENT_SYSTEM_PROMPT),
        CompletionModel::new(provider, params, &cfg.api_key),
    )
}

pub(super) fn assistant_agent() -> Agent {
    let cfg = &GLOBAL_CONFIG.model.as_ref().expect("No config");
    let provider: CompletionProvider = match cfg.provider {
        ModelProvider::OpenAi => OpenAiCompletionModel::Gpt4.into(),
        ModelProvider::Anthropic => AnthropicCompletionModel::Sonnet.into(),
    };
    let params = ModelParameters::default();
    Agent::new(
        Some(ASSISTANT_AGENT_SYSTEM_PROMPT),
        CompletionModel::new(provider, params, &cfg.api_key),
    )
}
