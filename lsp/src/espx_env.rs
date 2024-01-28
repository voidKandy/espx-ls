use espionox::{
    environment::agent::{
        language_models::LanguageModel,
        memory::{Message, MessageVector},
        AgentHandle,
    },
    Agent, Environment,
};
use tokio::runtime::Runtime;

use std::{
    env,
    sync::{Arc, Mutex, OnceLock},
    thread,
};

pub static ENVIRONMENT: OnceLock<Arc<Mutex<Environment>>> = OnceLock::new();
pub static MAIN_AGENT_HANDLE: OnceLock<Arc<Mutex<AgentHandle>>> = OnceLock::new();

pub async fn init_static_env_and_handle() {
    let (env, handle) = init_environment().await;
    _ = ENVIRONMENT.set(Arc::new(Mutex::new(env)));
    _ = MAIN_AGENT_HANDLE.set(Arc::new(Mutex::new(handle)));
    log::warn!("ENV AND MAIN AGENT HANDLE INITIALIZED");
}

const MAIN_AGENT_SYSTEM_PROMPT: &str = r#"#SILENT! DON'T TALK! JUST DO IT!

e**Important:** Your response should be in the form of pure, properly formatted Rust code. **CRITICAL:Do not include any markdown _or_ code block indicators (like rust or ).**

#EXAMPLE REQUEST
Create a simple "Hello, World!" script in Rust

#BEGIN EXAMPLE RESPONSE
fn main() {
    println!("Hello, World!");
}
#END EXAMPLE RESPONSE
----
#EMIT ONLY THE RAW TXT OF THE FILE CONTENT!"#;

const OBSERVER_SYSTEM_PROMPT: &str = r#"You are an observer of an Ai assistant.
    You will be given what the user is currently typing,
    you are expected to create a prompt for another model
    to try to finish what the user is in the process of typing."#;
const EXAMPLE_USER_INPUT: &str = r#"fn sum(nums: Vec<u64>) -> u64 { "#;
const EXAMPLE_OBSERVER_RESPONSE: &str =
    r#"The user is writing a function called sum that gets a sum from a vector of numbers"#;
const EXAMPLE_AGENT_OUTPUT: &str = r#"fn sum(nums: Vec<u64>) -> u64 {
     self.0.iter().fold(0, |mut sum, c| {
            sum += c.score();
            sum
        })
    }"#;

// Maybe this should just take an async closure
pub async fn prompt_main_agent(prompt: &str) -> Result<String, anyhow::Error> {
    let mut h = MAIN_AGENT_HANDLE
        .get()
        .expect("Can't get static agent")
        .lock()
        .expect("Can't lock static agent");

    let mut env = ENVIRONMENT
        .get()
        .expect("Can't get static env")
        .lock()
        .expect("Can't lock static env");

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

async fn init_environment() -> (Environment, AgentHandle) {
    dotenv::dotenv().ok();
    let api_key = env::var("OPENAI_API_KEY").ok();
    log::warn!("API KEY!: {:?}", api_key);
    let mut env = Environment::new(Some("main"), api_key.as_deref());
    let main_agent = Agent::new(MAIN_AGENT_SYSTEM_PROMPT, LanguageModel::default_gpt());
    let h = env
        .insert_agent(Some("main"), main_agent)
        .await
        .expect("Why couldn't it insert main agent?");
    (env, h)
}
