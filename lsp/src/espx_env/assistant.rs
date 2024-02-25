use anyhow::anyhow;
use espionox::{
    environment::agent::{
        language_models::{
            openai::gpt::{Gpt, GptModel},
            LanguageModel,
        },
        memory::{MessageRole, ToMessage},
    },
    Agent,
};
use lsp_types::Url;

use crate::doc_store::get_text_document;

use super::{get_watcher_memory_stream, io_prompt_agent, update_agent_cache, CopilotAgent};

const ASSISTANT_AGENT_SYSTEM_PROMPT: &str = r#"
You are an AI assistant in NeoVim. You will be provided with the user's codebase, as well as their most recent changes to the current file
answer their queries to the best of your ability. Your response should consider the language of the user's codebase and current document.
"#;

pub(super) fn assistant_agent() -> Agent {
    let gpt = LanguageModel::OpenAi(Gpt::new(GptModel::Gpt4, 0.6));
    Agent::new(ASSISTANT_AGENT_SYSTEM_PROMPT, gpt)
}

pub async fn prompt_from_file(url: &Url, prompt: impl ToMessage) -> Result<String, anyhow::Error> {
    let doc = get_text_document(&url).ok_or(anyhow!("No document at that URL"))?;
    update_agent_cache(doc, MessageRole::System, CopilotAgent::Assistant).await?;
    if let Some(mem_stream) = get_watcher_memory_stream().await.ok() {
        for mem in mem_stream.as_ref().into_iter() {
            update_agent_cache(
                mem.content.to_owned(),
                MessageRole::System,
                CopilotAgent::Assistant,
            )
            .await?;
        }
    }
    io_prompt_agent(prompt, CopilotAgent::Assistant).await
}
