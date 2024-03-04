use espionox::{
    agents::{
        independent::IndependentAgent,
        language_models::{
            openai::gpt::{Gpt, GptModel},
            LanguageModel,
        },
        memory::Message,
        Agent,
    },
    environment::{DispatchError, EnvError},
};

use super::ENVIRONMENT;

// Think about making this IndiAgent a static variable

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

async fn inde_sum_agent() -> Result<IndependentAgent, EnvError> {
    let gpt = LanguageModel::OpenAi(Gpt::new(GptModel::Gpt4, 0.6));
    let a = Agent::new(SUMMARIZER_AGENT_SYSTEM_PROMPT, gpt);
    let env = ENVIRONMENT.get().unwrap().lock().unwrap();
    env.make_agent_independent(a).await
}

pub async fn summarize(pre_content: Option<&str>, content: &str) -> Result<String, EnvError> {
    let mut i_agent = inde_sum_agent().await?;
    let mut c = pre_content.unwrap_or("").to_owned();

    c.push_str(&format!(" {}", content));
    let message = Message::new_user(&c);
    i_agent.agent.cache.push(message);
    i_agent.io_completion().await.map_err(|e| {
        let de: DispatchError = e.into();
        de.into()
    })
}
