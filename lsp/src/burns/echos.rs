use anyhow::anyhow;
use espionox::{agents::memory::MessageRole, environment::dispatch::EnvNotification};
use log::debug;
use lsp_types::{GotoDefinitionResponse, HoverContents, Location, Range, Url};
use rand::Rng;

use crate::{
    config::GLOBAL_CONFIG,
    espx_env::{agents::inner::InnerAgent, ENV_HANDLE},
};

use super::error::{BurnError, BurnResult};

/// Echo burns are PUT INTO the document BY the LSP
#[derive(Debug, Clone)]
pub(super) struct EchoBurn {
    pub(super) content: String,
    pub(super) hover_contents: HoverContents,
    pub(super) range: Range,
}

impl EchoBurn {
    pub fn generate_placeholder() -> String {
        let possible = vec![
            '∀', '∁', '∂', '∃', '∄', '∅', '∆', '∇', '∈', '∉', '∊', '∋', '∌', '∍', '∎', '∏', '∐',
            '∑', '−', '∓', '∔', '∕', '∖', '∗', '∘', '∙', '√', '∛', '∜', '∝', '∞', '∟', '∠', '∡',
            '∢', '∣', '∤', '∥', '∦', '∧', '∨', '∩', '∪', '∫', '∬', '∭', '∮', '∯', '∰', '∱', '∲',
            '∳', '∴', '∵', '∶', '∷', '∸', '∹', '∺', '∻', '∼', '∽', '∾', '∿', '≀', '≁', '≂', '≃',
            '≄', '≅', '≆', '≇', '≈', '≉', '≊', '≋', '≌', '≍', '≎', '≏', '≐', '≑', '≒', '≓', '≔',
            '≕', '≖', '≗', '≘', '≙', '≚', '≛', '≜', '≝', '≞', '≟', '≠', '≡', '≢', '≣', '≤', '≥',
            '≦', '≧', '≨', '≩', '≪', '≫', '≬', '≭', '≮', '≯', '≰', '≱', '≲', '≳', '≴', '≵', '≶',
            '≷', '≸', '≹', '≺', '≻', '≼', '≽', '≾', '≿', '⊀', '⊁', '⊂', '⊃', '⊄', '⊅', '⊆', '⊇',
            '⊈', '⊉', '⊊', '⊋', '⊌', '⊍', '⊎', '⊏', '⊐', '⊑', '⊒', '⊓', '⊔', '⊕', '⊖', '⊗', '⊘',
            '⊙', '⊚', '⊛', '⊜', '⊝', '⊞', '⊟', '⊠', '⊡', '⊢', '⊣', '⊤', '⊥', '⊦', '⊧', '⊨', '⊩',
            '⊪', '⊫', '⊬', '⊭', '⊮', '⊯', '⊰', '⊱', '⊲', '⊳', '⊴', '⊵', '⊹', '⊺', '⊻', '⊼', '⊽',
            '⊾', '⊿', '⋀', '⋁', '⋂', '⋃', '⋄', '⋅', '⋆', '⋇', '⋈', '⋉', '⋊', '⋋', '⋌', '⋍', '⋎',
            '⋏', '⋐', '⋑', '⋒', '⋓', '⋔', '⋕', '⋖', '⋗', '⋘', '⋙', '⋚', '⋛', '⋜', '⋝', '⋞', '⋟',
            '⋠', '⋡', '⋢', '⋣', '⋤', '⋥', '⋦', '⋧', '⋨', '⋩', '⋪', '⋫', '⋬', '⋭', '⋮', '⋯', '⋰',
            '⋱', '⋲', '⋳', '⋴', '⋵', '⋶', '⋷', '⋸', '⋹', '⋺', '⋻', '⋽', '⋾', '⋿',
        ];

        // let rand_indx = current_time.elapsed().unwrap().as_secs() as usize % (possible.len() - 1);
        let index = rand::thread_rng().gen_range(0..possible.len());
        possible[index].to_string()
    }

    pub async fn update_conversation_file(&self) -> BurnResult<()> {
        if let Some(notis) = &ENV_HANDLE.get().unwrap().lock().unwrap().notifications {
            if let Some(EnvNotification::AgentStateUpdate { cache, .. }) =
                notis.read().await.find_by(|s| {
                    s.iter().rev().find(|env_noti| {
                        if let EnvNotification::AgentStateUpdate { agent_id, .. } = env_noti {
                            agent_id == InnerAgent::Assistant.id()
                        } else {
                            false
                        }
                    })
                })
            {
                let mut out_string_vec = vec![];
                for message in cache.as_ref().into_iter() {
                    debug!("CONVERSATION UPDATE ITERATION: {}", message);
                    let role_str = {
                        if let MessageRole::Other { alias, .. } = &message.role {
                            alias.to_string()
                        } else {
                            message.role.to_string()
                        }
                    };
                    let role_str = convert_ascii(&role_str, '𝐀');
                    debug!("CONVERSATION UPDATE PUSHING: {}", role_str);
                    out_string_vec.push(format!("# {}\n\n", &role_str));

                    for chunk in split_message(&message.content, 100) {
                        out_string_vec.push(chunk);
                        out_string_vec.push(String::from("\n"));
                    }
                }
                let content_to_write = out_string_vec.join("");
                std::fs::write(
                    GLOBAL_CONFIG.paths.conversation_file_path.clone(),
                    content_to_write,
                )
                .unwrap();
                debug!("CONVERSATION FILE WRITTEN");
                return Ok(());
            }
            return Err(BurnError::Undefined(anyhow!(
                "No agent state update to write conversation file with"
            )));
        }

        Err(BurnError::Undefined(anyhow!(
            "No notifications in ENV_HANDLE"
        )))
    }

    pub fn goto_definition_response(&self) -> GotoDefinitionResponse {
        let path = &GLOBAL_CONFIG.paths.conversation_file_path;
        let path_str = format!("file:///{}", path.display().to_string());
        debug!("PATH STRING: [{}]", path_str);

        let uri = Url::parse(&path_str).expect("Failed to build LSP URL from tempfile path");
        lsp_types::GotoDefinitionResponse::Scalar(lsp_types::Location {
            uri,
            range: lsp_types::Range::default(),
        })
    }
}

// For making the role look 𝐍 𝐈 𝐂 𝐄
fn convert_ascii(str: &str, target: char) -> String {
    let start_code_point = target as u32;
    let str = str.to_lowercase();
    let mut chars = vec![' '];
    str.chars().for_each(|c| {
        let offset = c as u32 - 'a' as u32;
        chars.push(std::char::from_u32(start_code_point + offset).unwrap_or(c));
        chars.push(' ');
    });

    chars.into_iter().collect()
}

// For splitting the content of each message
fn split_message(message: &str, chunk_size: usize) -> Vec<String> {
    message
        .chars()
        .collect::<Vec<char>>()
        .chunks(chunk_size)
        .map(|chunk| chunk.iter().collect())
        .collect()
}
