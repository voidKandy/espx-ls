mod watcher;
use espionox::{
    environment::{
        agent::{
            language_models::{
                openai::gpt::streaming_utils::StreamedCompletionHandler, LanguageModel,
            },
            memory::{Message, MessageRole, MessageVector, ToMessage},
            AgentHandle,
        },
        dispatch::{AgentHashMap, ThreadSafeStreamCompletionHandler},
        Environment,
    },
    Agent,
};
use tokio::runtime::Runtime;

use std::{
    collections::HashMap,
    env,
    sync::{Arc, Mutex, OnceLock},
    thread,
};
pub use watcher::*;

pub static ENVIRONMENT: OnceLock<Arc<Mutex<Environment>>> = OnceLock::new();
pub static ASSISTANT_AGENT_HANDLE: OnceLock<Arc<Mutex<AgentHandle>>> = OnceLock::new();

#[derive(Debug, Hash, Eq, PartialEq)]
pub enum CopilotAgent {
    Assistant,
    Watcher,
}

impl CopilotAgent {
    fn id(&self) -> &str {
        match self {
            Self::Assistant => "assistant",
            Self::Watcher => "watcher",
        }
    }
}

pub async fn init_static_env_and_handle() {
    let mut env = init_environment().await;
    let _ = init_agent_handles(&mut env).await;
    _ = ENVIRONMENT.set(Arc::new(Mutex::new(env)));
    log::warn!("ENV AND CODE AGENT HANDLE INITIALIZED");
}
const ASSISTANT_AGENT_SYSTEM_PROMPT: &str = r#"
You are an AI assistant in NeoVim. You will be provided with the user's codebase, as well as their most recent changes to the current file
answer their queries to the best of your ability.
"#;

async fn init_environment() -> Environment {
    dotenv::dotenv().ok();
    let api_key = env::var("OPENAI_API_KEY").ok();
    log::warn!("API KEY!: {:?}", api_key);
    let env = Environment::new(Some("code"), api_key.as_deref());
    env
}

async fn init_agent_handles(env: &mut Environment) {
    let code_agent = Agent::new(ASSISTANT_AGENT_SYSTEM_PROMPT, LanguageModel::default_gpt());
    let id = CopilotAgent::Assistant;
    let _ = env
        .insert_agent(Some(id.id()), code_agent)
        .await
        .expect("Why couldn't it insert code agent?");

    let watcher_agent = Agent::new(WATCHER_AGENT_SYSTEM_PROMPT, LanguageModel::default_gpt());
    let id = CopilotAgent::Watcher;
    let _ = env
        .insert_agent(Some(id.id()), watcher_agent)
        .await
        .expect("Why couldn't it insert code agent?");
}

pub async fn io_prompt_agent(
    prompt: impl ToMessage,
    agent_id: CopilotAgent,
) -> Result<String, anyhow::Error> {
    let mut env = ENVIRONMENT
        .get()
        .expect("Can't get static env")
        .lock()
        .expect("Can't lock static env");
    let mut h = env
        .dispatch
        .read()
        .await
        .get_agent_handle(agent_id.id())
        .await?;

    log::info!("PROMPTING AGENT");
    if !env.is_running() {
        env.spawn().await.unwrap();
    }
    let ticket = h
        .request_io_completion(prompt.to_message(MessageRole::User))
        .await
        .unwrap();
    log::info!("Got ticket: {}", ticket);
    env.finalize_dispatch().await.unwrap();
    log::info!("Dispatch finalized");
    let noti = env
        .notifications
        .wait_for_notification(&ticket)
        .await
        .expect("Why couldn't it get the noti?");
    log::info!("Got notification: {:?}", noti);
    let response: &Message = noti.extract_body().try_into().unwrap();
    Ok(response.content.to_owned())
}

pub async fn stream_prompt_agent(
    prompt: impl ToMessage,
    agent_id: CopilotAgent,
) -> Result<ThreadSafeStreamCompletionHandler, anyhow::Error> {
    let mut env = ENVIRONMENT
        .get()
        .expect("Can't get static env")
        .lock()
        .expect("Can't lock static env");

    let mut h = env
        .dispatch
        .read()
        .await
        .get_agent_handle(agent_id.id())
        .await?;
    log::info!("PROMPTING AGENT");
    if !env.is_running() {
        env.spawn().await.unwrap();
    }
    let ticket = h
        .request_stream_completion(prompt.to_message(MessageRole::User))
        .await
        .unwrap();
    log::info!("Got ticket: {}", ticket);
    env.finalize_dispatch().await.unwrap();
    log::info!("Dispatch finalized");
    let noti = env
        .notifications
        .wait_for_notification(&ticket)
        .await
        .expect("Why couldn't it get the noti?");
    log::info!("Got notification: {:?}", noti);
    let response: &ThreadSafeStreamCompletionHandler = noti.extract_body().try_into()?;
    Ok(response.to_owned())
}
