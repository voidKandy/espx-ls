use anyhow::anyhow;
use espionox::{
    agents::{
        language_models::{
            openai::gpt::{Gpt, GptModel},
            LanguageModel,
        },
        memory::MessageStack,
        Agent,
    },
    environment::agent_handle::MessageRole,
};

use super::ENVIRONMENT;

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

pub(super) fn watcher_agent() -> Agent {
    let gpt = LanguageModel::OpenAi(Gpt::new(GptModel::Gpt4, 0.4));
    Agent::new(WATCHER_AGENT_SYSTEM_PROMPT, gpt)
}

pub async fn get_watcher_memory_stream() -> Result<MessageStack, anyhow::Error> {
    let mut env = ENVIRONMENT.get().unwrap().lock().unwrap();
    if !env.is_running() {
        env.spawn().await?;
    }
    let ticket = super::WATCHER_AGENT_HANDLE
        .get()
        .unwrap()
        .lock()
        .unwrap()
        .request_state()
        .await?;
    let noti = Box::new(env.notifications.wait_for_notification(&ticket).await?);
    let m: &MessageStack = noti.extract_body().try_into()?;
    let sans_sys_prompt: MessageStack = m.ref_filter_by(MessageRole::System, false).into();
    Ok(sans_sys_prompt)
}
