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
        errors::{DispatchError, EnvError},
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

use super::ENVIRONMENT;

pub static WATCHER_AGENT_HANDLE: OnceLock<Arc<Mutex<AgentHandle>>> = OnceLock::new();
pub const WATCHER_AGENT_SYSTEM_PROMPT: &str = r#"
#SILENT! DON'T TALK! JUST DO IT!
Provide description of what a developer is currenly doing provided their 
codebase, current file, and most recent changes.

Also provide a score of severity of the given changes. Where 10 is most severe and 0 is least.
"#;

pub async fn get_watcher_memory_stream() -> Result<MessageVector, anyhow::Error> {
    let mut env = ENVIRONMENT.get().unwrap().lock().unwrap();
    if !env.is_running() {
        env.spawn().await?;
    }
    let ticket = WATCHER_AGENT_HANDLE
        .get()
        .unwrap()
        .lock()
        .unwrap()
        .request_state()
        .await?;
    let noti = Box::new(env.notifications.wait_for_notification(&ticket).await?);
    let m: &MessageVector = noti.extract_body().try_into()?;
    Ok(m.clone())
}
