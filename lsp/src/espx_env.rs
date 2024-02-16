use espionox::{
    environment::{
        agent::{
            language_models::{
                openai::gpt::streaming_utils::StreamedCompletionHandler, LanguageModel,
            },
            memory::{Message, MessageVector},
            AgentHandle,
        },
        dispatch::{AgentHashMap, ThreadSafeStreamCompletionHandler},
    },
    Agent, Environment,
};
use tokio::runtime::Runtime;

use std::{
    collections::HashMap,
    env,
    sync::{Arc, Mutex, OnceLock},
    thread,
};

pub static ENVIRONMENT: OnceLock<Arc<Mutex<Environment>>> = OnceLock::new();
pub static CODE_AGENT_HANDLE: OnceLock<Arc<Mutex<AgentHandle>>> = OnceLock::new();
pub static WATCHER_AGENT_HANDLE: OnceLock<Arc<Mutex<AgentHandle>>> = OnceLock::new();

#[derive(Debug, Hash, Eq, PartialEq)]
pub enum CopilotAgent {
    Code,
    Watcher,
}

impl CopilotAgent {
    fn id(&self) -> &str {
        match self {
            Self::Code => "code",
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

const WATCHER_AGENT_SYSTEM_PROMPT: &str = r#"
    #SILENT! DON'T TALK! JUST DO IT!
    Provide descriptions of what a developer is currenly doing provided their 
    codebase, current file, and most recent changes.
"#;

const CODE_AGENT_SYSTEM_PROMPT: &str = r#"
#SILENT! DON'T TALK! JUST DO IT!
**Important:**
Your response should be in the form of pure, properly formatted code.
**CRITICAL:Do not include any markdown _or_ code block indicators**
#EXAMPLE REQUEST
Create a simple "Hello, World!" script in Rust
#BEGIN EXAMPLE RESPONSE
fn code() {
    println!("Hello, World!");
}
#END EXAMPLE RESPONSE
----
#EMIT ONLY THE RAW TXT OF THE FILE CONTENT!"#;

async fn init_environment() -> Environment {
    dotenv::dotenv().ok();
    let api_key = env::var("OPENAI_API_KEY").ok();
    log::warn!("API KEY!: {:?}", api_key);
    let env = Environment::new(Some("code"), api_key.as_deref());
    env
}

async fn init_agent_handles(env: &mut Environment) {
    let code_agent = Agent::new(CODE_AGENT_SYSTEM_PROMPT, LanguageModel::default_gpt());
    let id = CopilotAgent::Code;
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
    prompt: &str,
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
        .request_io_completion(Message::new_user(prompt))
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
    prompt: &str,
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
        .request_stream_completion(Message::new_user(prompt))
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
