use crate::config::GLOBAL_CONFIG;
use espionox::{
    agents::Agent,
    language_models::{
        anthropic::AnthropicCompletionHandler, inference::LLMCompletionHandler,
        openai::completions::OpenAiCompletionHandler, ModelProvider, LLM,
    },
};

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
    let handler: LLMCompletionHandler = match cfg.provider {
        ModelProvider::OpenAi => OpenAiCompletionHandler::Gpt3.into(),
        ModelProvider::Anthropic => AnthropicCompletionHandler::Haiku.into(),
    };
    let llm = LLM::new_completion_model(handler, None, &cfg.api_key);
    Agent::new(Some(SUMMARIZER_AGENT_SYSTEM_PROMPT), llm)
}

pub(super) fn assistant_agent() -> Agent {
    let cfg = &GLOBAL_CONFIG.model.as_ref().expect("No config");
    let handler: LLMCompletionHandler = match cfg.provider {
        ModelProvider::OpenAi => OpenAiCompletionHandler::Gpt4.into(),
        ModelProvider::Anthropic => AnthropicCompletionHandler::Sonnet.into(),
    };
    let llm = LLM::new_completion_model(handler, None, &cfg.api_key);
    Agent::new(Some(ASSISTANT_AGENT_SYSTEM_PROMPT), llm)
}
