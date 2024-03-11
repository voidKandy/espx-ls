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

// #[derive(Debug)]
// pub struct DocStoreRAG {
//     id_of_agent_to_update: String,
//     store_agent: IndependentAgent,
// }
//
// impl DocStoreRAG {
//     pub async fn new(id: &str) -> Result<Self, EnvError> {
//         let env = ENVIRONMENT.get().unwrap().lock().unwrap();
//         let a = Agent::new("", LanguageModel::default_gpt());
//         let store_agent = env.make_agent_independent(a).await?;
//
//         Ok(Self {
//             store_agent,
//             id_of_agent_to_update: id.to_owned(),
//         })
//     }
//
//     pub fn parse_function_response_to_urls(response: Value) -> Result<Vec<Url>, anyhow::Error> {
//         let val = response
//             .get("urls")
//             .ok_or(anyhow!("Could not get urls from response"))?;
//         let str: String = serde_json::from_value(val.to_owned())?;
//         Ok(str.split(',').filter_map(|s| Url::parse(s).ok()).collect())
//     }
//
//     pub async fn function_prompt(
//         &mut self,
//         user_prompt: &str,
//         docs_map: HashMap<&Url, &Document>,
//     ) -> Result<Value, AgentError> {
//         let message =
//             Self::prepare_function_prompt(user_prompt, docs_map).to_message(MessageRole::User);
//         self.store_agent.agent.cache.push(message);
//         self.store_agent
//             .function_completion(self.url_array_from_prompt_function())
//             .await
//     }
//
//     fn url_array_from_prompt_function(&self) -> CustomFunction {
//         let location_info = PropertyInfo::new("url_array", json!("The list of document urls"));
//
//         let location_prop = Property::build_from("urls")
//             .return_type("string")
//             .add_info(location_info)
//             .finished();
//
//         CustomFunction::build_from("get_url_array_from_prompt")
//             .description(
//                 r#"
//                  Given a user prompt and information on multiple documents,
//                  return an object with an array of the urls of all documents
//                  that might be relevant to the user's query.
//                  If no documents are relevant, return an empty array."#,
//             )
//             .add_property(location_prop, true)
//             .finished()
//     }
//
//     fn prepare_function_prompt(user_prompt: &str, docs_map: HashMap<&Url, &Document>) -> String {
//         let docs_strings = docs_map.into_iter().fold(vec![], |mut strs, (url, doc)| {
//             strs.push(format!(
//                 "DOCUMENT URL: [{}] DOCUMENT SUMMARY: [{:?}]",
//                 url, doc.summary
//             ));
//             strs
//         });
//         format!(
//             "USER PROMPT: [{}], DOCUMENTS INFO: [{}]",
//             user_prompt,
//             docs_strings.join(",")
//         )
//     }
// }
//
// impl EnvListener for DocStoreRAG {
//     fn method<'l>(
//         &'l mut self,
//         trigger_message: EnvMessage,
//         dispatch: &'l mut Dispatch,
//     ) -> ListenerMethodReturn {
//         Box::pin(async {
//             // if let Some(_) = match &trigger_message {
//             //     EnvMessage::Request(req) => match req {
//             //         EnvRequest::GetCompletion { agent_id, .. } => {
//             //             if agent_id == &self.id_of_agent_to_update {
//             //                 Some(())
//             //             } else {
//             //                 None
//             //             }
//             //         }
//             //         _ => None,
//             //     },
//             //     _ => None,
//             // } {
//             //     // let gstore = GLOBAL_STORE.get().unwrap().lock().unwrap();
//             //
//             //     let agent = dispatch
//             //         .get_agent_mut(&self.id_of_agent_to_update)
//             //         .map_err(|_| ListenerError::NoAgent)?;
//             //     let mut last_user_message: Message = agent
//             //         .cache
//             //         .pop(Some(MessageRole::User))
//             //         .expect("No last user message");
//             //     self.store_agent.agent.cache.push(Message::new_system("
//             //           You will be given a user prompt. Considering their question, give a general description of what might
//             //           be contained in a code file that would be relavent to their question."
//             //     ));
//             //     self.store_agent
//             //         .agent
//             //         .cache
//             //         .push(Message::new_user(&last_user_message.content));
//             //     let response = self
//             //         .store_agent
//             //         .io_completion()
//             //         .await
//             //         .expect("Failed to get IO completion of store agent");
//             //     let agent_response_emb = EmbeddingVector::from(embed(&response)?);
//             //
//             //     // NEED TO PUT A MUTEX TO GLOBAL STORE IN THIS STRUCT
//             //     let docs_map = self
//             //         .gstore
//             //         .documents
//             //         .get_by_proximity(agent_response_emb, 0.3);
//             //
//             //     last_user_message.content =
//             //         Self::prepare_function_prompt(&last_user_message.content, docs_map);
//             //     agent.cache.push(last_user_message);
//             //     let new_message = return Ok(trigger_message);
//             // }
//             // Err(ListenerError::IncorrectTrigger)
//             Ok(trigger_message)
//         })
//     }
//     fn trigger<'l>(&self, env_message: &'l EnvMessage) -> Option<&'l EnvMessage> {
//         if let EnvMessage::Request(req) = env_message {
//             if let EnvRequest::GetCompletion { agent_id, .. } = req {
//                 if agent_id == &self.id_of_agent_to_update {
//                     return Some(env_message);
//                 }
//             }
//         }
//         None
//     }
// }
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
