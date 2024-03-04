mod assistant;
mod summarizer;
mod watcher;
use anyhow::anyhow;
use espionox::{
    agents::memory::{Message, MessageRole, ToMessage},
    environment::{
        agent_handle::AgentHandle,
        dispatch::{EnvNotification, ThreadSafeStreamCompletionHandler},
        Environment,
    },
};

pub use assistant::*;
use std::{
    env,
    sync::{Arc, Mutex, OnceLock},
};
pub use summarizer::*;
pub use watcher::*;

pub static ENVIRONMENT: OnceLock<Arc<Mutex<Environment>>> = OnceLock::new();
pub static ASSISTANT_AGENT_HANDLE: OnceLock<Arc<Mutex<AgentHandle>>> = OnceLock::new();
pub static WATCHER_AGENT_HANDLE: OnceLock<Arc<Mutex<AgentHandle>>> = OnceLock::new();

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

async fn init_environment() -> Environment {
    dotenv::dotenv().ok();
    let api_key = env::var("OPENAI_API_KEY").ok();
    log::warn!("API KEY!: {:?}", api_key);
    let env = Environment::new(Some("code"), api_key.as_deref());
    env
}

async fn init_agent_handles(env: &mut Environment) {
    let assistant = env
        .insert_agent(Some(CopilotAgent::Assistant.id()), assistant_agent())
        .await
        .expect("Why couldn't it insert code agent?");
    let _ = ASSISTANT_AGENT_HANDLE.set(Arc::new(Mutex::new(assistant)));

    let watcher = env
        .insert_agent(Some(CopilotAgent::Watcher.id()), watcher_agent())
        .await
        .expect("Why couldn't it insert code agent?");
    let _ = WATCHER_AGENT_HANDLE.set(Arc::new(Mutex::new(watcher)));
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

    if !env.is_running() {
        env.spawn().await.unwrap();
    }

    let mut h = match agent_id {
        CopilotAgent::Assistant => ASSISTANT_AGENT_HANDLE.get().unwrap().lock().unwrap(),
        CopilotAgent::Watcher => WATCHER_AGENT_HANDLE.get().unwrap().lock().unwrap(),
    };

    log::info!("PROMPTING AGENT");
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

    if !env.is_running() {
        env.spawn().await.unwrap();
    }

    let mut h = match agent_id {
        CopilotAgent::Assistant => ASSISTANT_AGENT_HANDLE.get().unwrap().lock().unwrap(),
        CopilotAgent::Watcher => WATCHER_AGENT_HANDLE.get().unwrap().lock().unwrap(),
    };
    log::info!("PROMPTING AGENT");
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

pub async fn update_agent_cache(
    to_message: impl ToMessage,
    role: MessageRole,
    agent_id: CopilotAgent,
) -> Result<(), anyhow::Error> {
    let mut env = ENVIRONMENT
        .get()
        .expect("Can't get static env")
        .lock()
        .expect("Can't lock static env");

    if !env.is_running() {
        env.spawn().await?;
    }

    let mut h = match agent_id {
        CopilotAgent::Assistant => ASSISTANT_AGENT_HANDLE.get().unwrap().lock().unwrap(),
        CopilotAgent::Watcher => WATCHER_AGENT_HANDLE.get().unwrap().lock().unwrap(),
    };
    let _ = h.request_cache_push(to_message, role).await;
    env.finalize_dispatch().await?;
    let noti_stack = env.notifications.0.read().await;
    match noti_stack
        .front()
        .ok_or(anyhow!("Notification stack is empty"))?
    {
        &EnvNotification::AgentStateUpdate { .. } => Ok(()),
        noti => Err(anyhow!("Unexpected front notification: {:?}", noti)),
    }
}
