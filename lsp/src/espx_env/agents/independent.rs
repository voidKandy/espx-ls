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
    Embedder,
}
pub fn all_indies() -> Vec<(IndyAgent, Agent)> {
    vec![sum_agent(), embedding_agent()]
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

fn embedding_agent() -> (IndyAgent, Agent) {
    let gpt = OpenAiEmbeddingModel::Ada;
    let handler = LLM::new_embedding_model(gpt.into(), None);
    (IndyAgent::Summarizer, Agent::new(None, handler))
}

// pub async fn summarize(
//     pre_content: Option<&str>,
//     content: &str,
// ) -> Result<String, EnvError> {
//     let mut i_agent: IndependentAgent<H> = inde_sum_agent().await?;
//     let mut c = pre_content.unwrap_or("").to_owned();
//
//     c.push_str(&format!(" {}", content));
//     let message = Message::new_user(&c);
//     i_agent.agent.cache.push(message);
//     i_agent.io_completion().await.map_err(|e| {
//         let de: DispatchError = e.into();
//         de.into()
//     })
// }
