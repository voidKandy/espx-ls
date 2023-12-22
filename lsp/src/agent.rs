use espionox::{
    agents::{
        spo_agents::{observer::ObservationStep, AgentObserver, ObservationProtocol},
        Agent,
    },
    language_models::{openai::gpt::GptModel, LanguageModel},
    memory::{Memory, Message, MessageRole, MessageVector},
};
use std::{
    sync::{Arc, Mutex, OnceLock},
    thread,
};
use tokio::runtime::Runtime;

pub static AGENT: OnceLock<Arc<Mutex<Agent>>> = OnceLock::new();
pub fn init_agent() {
    _ = AGENT.set(Arc::new(Mutex::new(main_agent())));
    log::warn!("AGENT INITIALIZED");
}

const MAIN_AGENT_SYSTEM_PROMPT: &str = r#"#SILENT! DON'T TALK! JUST DO IT!

**Important:** Your response should be in the form of pure, properly formatted Rust code. **CRITICAL:Do not include any markdown _or_ code block indicators (like rust or ).**

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

fn observer() -> AgentObserver {
    let obs_prompt = MessageVector::from(vec![
        Message::new_standard(MessageRole::System, OBSERVER_SYSTEM_PROMPT),
        Message::new_standard(MessageRole::User, EXAMPLE_USER_INPUT),
        Message::new_standard(MessageRole::Assistant, EXAMPLE_OBSERVER_RESPONSE),
    ]);
    let memory = Memory::build()
        .init_prompt(obs_prompt)
        .caching_mechanism(espionox::memory::CachingMechanism::Forgetful)
        .finished();
    let model = LanguageModel::default_gpt();
    let agent = Agent {
        memory,
        model,
        observer: None,
    };
    let step = ObservationStep::BeforePrompt;
    let protocol = ObservationProtocol {
        mutate_input: Some(step),
        mutate_agent: None,
    };
    AgentObserver::from_agent(agent, protocol)
}

fn main_agent() -> Agent {
    let init_prompt = MessageVector::from(vec![
        Message::new_standard(MessageRole::System, MAIN_AGENT_SYSTEM_PROMPT),
        Message::new_standard(MessageRole::User, EXAMPLE_OBSERVER_RESPONSE),
        Message::new_standard(MessageRole::Assistant, EXAMPLE_AGENT_OUTPUT),
    ]);
    let memory = Memory::build().init_prompt(init_prompt).finished();
    let model = LanguageModel::new_gpt(GptModel::Gpt4, 0.4);
    Agent {
        memory,
        model,
        observer: Some(observer()),
    }
}

pub fn block_prompt(prompt: &str) -> Result<String, anyhow::Error> {
    let mut a = AGENT
        .get()
        .expect("Can't get static agent")
        .lock()
        .expect("Can't lock static agent");
    let rt = Runtime::new()?;
    log::info!("PROMPTING AGENT");
    let response = rt.block_on(async move {
        let r = a
            .prompt(prompt.to_string())
            .await
            .expect("Failed to get completion");
        log::info!("AGENT REPONDED: {}", r);
        r
    });
    Ok(response)
}

/// Only gets one for now
fn get_predictions(prompt: &str) -> Vec<String> {
    let prediction = block_prompt(prompt).unwrap();
    vec![prediction]
}

#[cfg(test)]
mod tests {
    use crate::agent::{EXAMPLE_OBSERVER_RESPONSE, EXAMPLE_USER_INPUT};

    use super::main_agent;
    use tokio::runtime::Runtime;

    #[test]
    fn predictions_prefix_match_input() {
        let mut agent = main_agent();
        let rt = Runtime::new().unwrap();

        let response = rt.block_on(async move {
            let r = agent
                .prompt(EXAMPLE_OBSERVER_RESPONSE.to_string())
                .await
                .expect("Failed to get completion");
            log::info!("AGENT REPONDED: {}", r);
            r
        });
        let len = EXAMPLE_USER_INPUT.len();
        assert_eq!(&response[0..len], EXAMPLE_USER_INPUT)
    }
}
