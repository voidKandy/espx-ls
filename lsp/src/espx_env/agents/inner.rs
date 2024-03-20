use std::sync::{Arc, Mutex};

use espionox::{
    agents::{
        memory::{Message, MessageStack, ToMessage},
        Agent, AgentError,
    },
    environment::{agent_handle::AgentHandle, env_handle::EnvHandle},
    language_models::{openai::completions::OpenAiCompletionHandler, ModelParameters, LLM},
};

const ASSISTANT_AGENT_SYSTEM_PROMPT: &str = r#"
You are an AI assistant in NeoVim. You will be provided with the user's codebase, as well as their most recent changes to the current file
answer their queries to the best of your ability. Your response should consider the language of the user's codebase and current document.
"#;

const SANITIZER_AGENT_SYSTEM_PROMPT: &str = r#"
You are a sanitizer agent. You will be given text to sanitize or adjust based on 
the needs of the user.
"#;

const WATCHER_AGENT_SYSTEM_PROMPT: &str = r#"
#SILENT! DON'T TALK! JUST DO IT!
Provide description of what a developer is currenly doing provided their 
codebase, current file, and most recent changes.

Also provide a score of severity of the given changes. Where 10 is most severe and 0 is least.
*BEGIN EXAMPLE OUTPUT*
[CHANGE ON LINE >>>LINE NUMBER<<<]
[SEVERITY: >>>SOME NUMBER BETWEEN 0 and 10<<<]
>>>ACTUAL SUMMARY OF CHANGE<<< 
[END OF CHANGE ON LINE >>>LINE NUMBER<<<]
*END EXAMPLE OUTPUT*
"#;

#[derive(Debug, Hash, Eq, PartialEq)]
pub enum InnerAgent {
    Assistant,
    Sanitizer,
    Watcher,
}

pub fn all_inner_agents() -> Vec<(InnerAgent, Agent)> {
    vec![assistant_agent(), sanitizer_agent(), watcher_agent()]
}

fn assistant_agent() -> (InnerAgent, Agent) {
    let params = ModelParameters {
        temperature: Some(60),
        ..Default::default()
    };
    let gpt = OpenAiCompletionHandler::Gpt4;

    let handler = LLM::new_completion_model(gpt.into(), Some(params));
    (
        InnerAgent::Assistant,
        Agent::new(Some(ASSISTANT_AGENT_SYSTEM_PROMPT), handler),
    )
}

fn sanitizer_agent() -> (InnerAgent, Agent) {
    let params = ModelParameters {
        temperature: Some(60),
        ..Default::default()
    };
    let gpt = OpenAiCompletionHandler::Gpt4;
    let handler = LLM::new_completion_model(gpt.into(), Some(params));
    (
        InnerAgent::Sanitizer,
        Agent::new(Some(SANITIZER_AGENT_SYSTEM_PROMPT), handler),
    )
}

fn watcher_agent() -> (InnerAgent, Agent) {
    let params = ModelParameters {
        temperature: Some(40),
        ..Default::default()
    };
    let gpt = OpenAiCompletionHandler::Gpt4;
    let handler = LLM::new_completion_model(gpt.into(), Some(params));
    (
        InnerAgent::Watcher,
        Agent::new(Some(WATCHER_AGENT_SYSTEM_PROMPT), handler),
    )
}

impl InnerAgent {
    pub fn id(&self) -> &str {
        match self {
            Self::Assistant => "assistant",
            Self::Sanitizer => "sanitizer",
            Self::Watcher => "watcher",
        }
    }
}

// pub async fn get_watcher_memory_stream() -> Result<MessageStack, anyhow::Error> {
//     let mut env = ENVIRONMENT.get().unwrap().lock().unwrap();
//     if !env.is_running() {
//         env.spawn().await?;
//     }
//     let ticket = super::WATCHER_AGENT_HANDLE
//         .get()
//         .unwrap()
//         .lock()
//         .unwrap()
//         .request_state()
//         .await?;
//     let noti = Box::new(env.notifications.wait_for_notification(&ticket).await?);
//     let m: &MessageStack = noti.extract_body().try_into()?;
//     let sans_sys_prompt: MessageStack = m.ref_filter_by(MessageRole::System, false).into();
//     Ok(sans_sys_prompt)
// }
