use std::{collections::HashMap, env};

use anyhow::anyhow;
use espionox::{
    agents::{
        independent::IndependentAgent,
        language_models::{
            embed,
            openai::{
                functions::{CustomFunction, Property, PropertyInfo},
                gpt::{Gpt, GptModel},
            },
            LanguageModel,
        },
        memory::{embeddings::EmbeddingVector, MessageRole, ToMessage},
        Agent, AgentError,
    },
    environment::{
        agent_handle::Message,
        dispatch::{
            listeners::{error::ListenerError, ListenerMethodReturn},
            Dispatch, EnvListener, EnvMessage, EnvRequest,
        },
        EnvError,
    },
};
use lsp_types::Url;
use serde_json::{json, Value};

use crate::store::{get_text_document, Action, Document, GlobalStore, GLOBAL_STORE};

use super::{
    get_watcher_memory_stream, io_prompt_agent, update_agent_cache, CopilotAgent, ENVIRONMENT,
};

const ASSISTANT_AGENT_SYSTEM_PROMPT: &str = r#"
You are an AI assistant in NeoVim. You will be provided with the user's codebase, as well as their most recent changes to the current file
answer their queries to the best of your ability. Your response should consider the language of the user's codebase and current document.
"#;

pub(super) fn assistant_agent() -> Agent {
    let gpt = LanguageModel::OpenAi(Gpt::new(GptModel::Gpt4, 0.6));
    Agent::new(ASSISTANT_AGENT_SYSTEM_PROMPT, gpt)
}

pub async fn prompt_from_file(prompt: impl ToMessage) -> Result<String, anyhow::Error> {
    io_prompt_agent(prompt, CopilotAgent::Assistant).await
}

#[cfg(test)]
mod tests {
    // use super::DocStoreRAG;
    use lsp_types::Url;
    use std::collections::HashMap;

    use crate::{espx_env::CopilotAgent, store::Document};

    #[tokio::test]
    async fn store_function_prompt_works() {
        // println!("Building listener");
        // let user_prompt = "How do i rewrite the function sqrt to take an extra perameter?";
        // let mut docs_map = HashMap::new();
        // let url = Url::parse("file:///tmp/foo").unwrap();
        // let doc = Document {
        //     chunks: vec![],
        //     url: Url::parse("file:///tmp/foo").unwrap(),
        //     summary: "This file covers all math related functions".to_string(),
        // };
        // docs_map.insert(&url, &doc);
        // let url = Url::parse("file:///algo/boo").unwrap();
        // let doc = Document {
        //     chunks: vec![],
        //     url: Url::parse("file:///algo/boo").unwrap(),
        //     summary: "This file covers all geology functions".to_string(),
        // };
        // docs_map.insert(&url, &doc);
        // let mut listener = DocStoreRAG::new(CopilotAgent::Assistant.id());
        // let response = listener
        //     .function_prompt(user_prompt, docs_map)
        //     .await
        //     .unwrap();
        // let parsed = DocStoreRAG::parse_function_response_to_urls(response);
        // assert!(parsed.is_ok())
    }
}
