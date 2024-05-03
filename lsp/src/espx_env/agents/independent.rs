use espionox::{
    agents::Agent,
    language_models::{
        openai::{completions::OpenAiCompletionHandler, embeddings::OpenAiEmbeddingModel},
        LLM,
    },
};

#[derive(Debug, Hash, Eq, PartialEq)]
pub enum IndyAgent {
    Summarizer,
}
pub fn all_indies() -> Vec<(IndyAgent, Agent)> {
    vec![sum_agent()]
}

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

fn sum_agent() -> (IndyAgent, Agent) {
    let gpt = OpenAiCompletionHandler::Gpt4;
    let handler = LLM::new_completion_model(gpt.into(), None);
    (
        IndyAgent::Summarizer,
        Agent::new(Some(SUMMARIZER_AGENT_SYSTEM_PROMPT), handler),
    )
}
