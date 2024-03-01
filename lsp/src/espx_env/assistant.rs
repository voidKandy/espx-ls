use std::{collections::HashMap, env};

use anyhow::anyhow;
use espionox::{
    environment::{
        agent::{
            language_models::{
                embed,
                openai::{
                    functions::{CustomFunction, Property, PropertyInfo},
                    gpt::{Gpt, GptModel},
                },
                LanguageModel,
            },
            memory::{embeddings::EmbeddingVector, MessageRole, ToMessage},
        },
        dispatch::{Dispatch, EnvListener, EnvMessage, EnvRequest, ListenerError},
    },
    Agent,
};
use lsp_types::Url;
use serde_json::{json, Value};

use crate::store::{get_text_document, Action, Document, GlobalStore, GLOBAL_STORE};

use super::{get_watcher_memory_stream, io_prompt_agent, update_agent_cache, CopilotAgent};

const ASSISTANT_AGENT_SYSTEM_PROMPT: &str = r#"
You are an AI assistant in NeoVim. You will be provided with the user's codebase, as well as their most recent changes to the current file
answer their queries to the best of your ability. Your response should consider the language of the user's codebase and current document.
"#;

#[derive(Debug)]
pub struct UpdateFromStore {
    store_agent: Agent,
}

impl UpdateFromStore {
    pub fn new() -> Self {
        let store_agent = Agent::new("", LanguageModel::default_gpt());
        Self { store_agent }
    }

    pub fn parse_function_response_to_urls(response: Value) -> Result<Vec<Url>, anyhow::Error> {
        let val = response
            .get("urls")
            .ok_or(anyhow!("Could not get urls from response"))?;
        let str: String = serde_json::from_value(val.to_owned())?;
        Ok(str.split(',').filter_map(|s| Url::parse(s).ok()).collect())
    }

    pub async fn function_prompt(
        &mut self,
        user_prompt: &str,
        docs_map: HashMap<&Url, &Document>,
    ) -> Result<Value, anyhow::Error> {
        let client = reqwest::Client::new();
        println!("Client fine");

        let message =
            Self::prepare_function_prompt(user_prompt, docs_map).to_message(MessageRole::User);
        println!("prepped");
        self.store_agent.cache.push(message);
        let context: Vec<Value> = (&self.store_agent.cache).into();

        let api_key = env::var("OPENAI_API_KEY")?;
        println!("Got key");
        let res = self.store_agent.model.function_completion_fn()(
            &client,
            &api_key,
            &context,
            &self.store_agent.model,
            &self.url_array_from_prompt_function().function(),
        )
        .await
        .unwrap();
        res.parse_fn()
    }

    fn url_array_from_prompt_function(&self) -> CustomFunction {
        let location_info = PropertyInfo::new("url_array", json!("The list of document urls"));

        let location_prop = Property::build_from("urls")
            .return_type("string")
            .add_info(location_info)
            .finished();

        CustomFunction::build_from("get_url_array_from_prompt")
            .description(
                r#"
                 Given a user prompt and information on multiple documents,
                 return an object with an array of the urls of all documents
                 that might be relevant to the user's query.
                 If no documents are relevant, return an empty array."#,
            )
            .add_property(location_prop, true)
            .finished()
    }

    fn prepare_function_prompt(user_prompt: &str, docs_map: HashMap<&Url, &Document>) -> String {
        let docs_strings = docs_map.into_iter().fold(vec![], |mut strs, (url, doc)| {
            strs.push(format!(
                "DOCUMENT URL: [{}] DOCUMENT SUMMARY: [{}]",
                url, doc.summary
            ));
            strs
        });
        format!(
            "USER PROMPT: [{}], DOCUMENTS INFO: [{}]",
            user_prompt,
            docs_strings.join(",")
        )
    }
}

impl EnvListener for UpdateFromStore {
    fn method<'l>(
        &'l mut self,
        trigger_message: EnvMessage,
        dispatch: &'l mut Dispatch,
    ) -> espionox::environment::dispatch::ListenerMethodReturn {
        Box::pin(async move {
            let agent = dispatch.get_agent_mut(CopilotAgent::Assistant.id())?;
            let prompt = &agent
                .cache
                .as_ref()
                .iter()
                .rfind(|m| m.role == MessageRole::User)
                .ok_or(ListenerError::Other(
                    "Most recent User message returned none".to_string(),
                ))?
                .content;
            let prompt_embedding = EmbeddingVector::from(embed(&prompt)?);
            let store = GLOBAL_STORE.get().unwrap().lock().unwrap();

            Ok(trigger_message)
        })
    }

    fn trigger<'l>(&self, env_message: &'l EnvMessage) -> Option<&'l EnvMessage> {
        if let EnvMessage::Request(req) = env_message {
            match req {
                EnvRequest::GetCompletion { agent_id, .. }
                | EnvRequest::GetCompletionStreamHandle { agent_id, .. } => {
                    if agent_id == CopilotAgent::Assistant.id() {
                        return Some(env_message);
                    }
                }
                _ => {}
            }
        }

        None
    }
}

pub(super) fn assistant_agent() -> Agent {
    let gpt = LanguageModel::OpenAi(Gpt::new(GptModel::Gpt4, 0.6));
    Agent::new(ASSISTANT_AGENT_SYSTEM_PROMPT, gpt)
}

pub async fn prompt_from_file(prompt: impl ToMessage) -> Result<String, anyhow::Error> {
    io_prompt_agent(prompt, CopilotAgent::Assistant).await
}

#[cfg(test)]
mod tests {
    use super::UpdateFromStore;
    use lsp_types::Url;
    use std::collections::HashMap;

    use crate::store::Document;

    #[tokio::test]
    async fn store_function_prompt_works() {
        println!("Building listener");
        let user_prompt = "How do i rewrite the function sqrt to take an extra perameter?";
        let mut docs_map = HashMap::new();
        let url = Url::parse("file:///tmp/foo").unwrap();
        let doc = Document {
            url: "file:///tmp/foo".to_string(),
            changes: HashMap::new(),
            current_text: "String".to_string(),
            summary: "This file covers all math related functions".to_string(),
        };
        docs_map.insert(&url, &doc);
        let url = Url::parse("file:///algo/boo").unwrap();
        let doc = Document {
            url: "file:///algo/boo".to_string(),
            changes: HashMap::new(),
            current_text: "String".to_string(),
            summary: "This file covers all geology functions".to_string(),
        };
        docs_map.insert(&url, &doc);
        let mut listener = UpdateFromStore::new();
        let response = listener
            .function_prompt(user_prompt, docs_map)
            .await
            .unwrap();
        let parsed = UpdateFromStore::parse_function_response_to_urls(response);
        assert!(parsed.is_ok())
    }
}
