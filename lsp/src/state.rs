use crate::{
    agents::{error::AgentsResult, Agents},
    config::Config,
    database::Database,
    interact::{
        id::{InteractID, COMMAND_MASK, DOCUMENT_CHARACTER, GLOBAL_CHARACTER, PUSH_ID, SCOPE_MASK},
        lexer::{Lexer, Token, TokenVec},
        registry::InteractRegistry,
    },
};
use anyhow::anyhow;
use espionox::{
    agents::{memory::OtherRoleTo, Agent},
    prelude::{Message, MessageRole},
};
use lsp_types::Uri;
use std::{collections::HashMap, sync::Arc};
use tokio::sync::{RwLock, RwLockReadGuard, RwLockWriteGuard};
use tracing::warn;

pub struct SharedState(Arc<RwLock<LspState>>);

#[derive(Debug)]
pub struct LspState {
    pub documents: HashMap<Uri, TokenVec>,
    pub database: Option<Database>,
    pub registry: InteractRegistry,
    pub agents: Option<Agents>,
}

impl LspState {
    async fn new(mut config: Config) -> anyhow::Result<Self> {
        let database = Database::init(&mut config).await.ok();
        let mut agents = config.model.take().and_then(|cfg| Some(Agents::from(cfg)));
        let mut registry = InteractRegistry::default();
        if let Some(ref scopes_config) = &config.scopes {
            for (char, scope_settings) in scopes_config.clone().into_iter() {
                registry.register_scope(&char)?;
                if let Some(agents) = agents.as_mut() {
                    agents.create_custom_agent(char, scope_settings.sys_prompt);
                }
            }
        }

        Ok(Self {
            documents: HashMap::new(),
            registry,
            database,
            agents,
        })
    }

    pub fn agent_mut_from_interact_integer(
        &mut self,
        integer: u8,
        current_document_uri: &Uri,
    ) -> AgentsResult<&mut Agent> {
        let agents = self
            .agents
            .as_mut()
            .ok_or(anyhow!("agents not present in state"))?;
        let masked = integer & SCOPE_MASK;
        let char = self
            .registry
            .get_interact_char(InteractID::Scope(masked))
            .ok_or(anyhow!(
                "registry does not have char for id: {integer} with mask: {SCOPE_MASK}"
            ))?;
        match char {
            _ if char == &DOCUMENT_CHARACTER => agents.doc_agent_mut(current_document_uri),
            _ if char == &GLOBAL_CHARACTER => Ok(agents.global_agent_mut()),
            custom_character => agents.custom_agent_mut(*custom_character.as_ref()),
        }
    }

    pub fn update_doc_and_agents_from_text(
        &mut self,
        uri: Uri,
        text: String,
    ) -> anyhow::Result<()> {
        if let Some(agents) = self.agents.as_mut() {
            agents.update_or_create_doc_agent(&uri, &text);
        }

        let uri_str = uri.as_str().to_string();
        let ext = &uri_str
            .rsplit_once('.')
            .expect("uri does not have extension")
            .1;
        let mut lexer = Lexer::new(&text, ext);
        let new_tokens = lexer.lex_input(&self.registry);
        let old_tokens = self.documents.get(&uri);
        let mut prev_existing_push_scopes = old_tokens
            .and_then(|tokens| {
                let mut all = vec![];
                for idx in tokens.comment_indices() {
                    let mut iter = tokens.as_ref().iter();
                    if let Token::Comment(comment) = iter.nth(*idx).unwrap() {
                        if let Some(integer) = comment.try_get_interact_integer().ok() {
                            warn!("id: {integer:?}");
                            // agent.cache.mut_filter_by(&role, false);
                            if integer & COMMAND_MASK == *PUSH_ID.as_ref() {
                                all.push(integer & SCOPE_MASK)
                            }
                        }
                    }
                }
                Some(all)
            })
            .unwrap_or(vec![]);

        let role = MessageRole::Other {
            alias: uri.to_string(),
            coerce_to: OtherRoleTo::User,
        };

        for comment_idx in new_tokens.comment_indices() {
            let mut iter = new_tokens.as_ref().iter();
            if let Token::Comment(comment) = iter.nth(*comment_idx).unwrap() {
                warn!("comment: {comment:?}");
                if let Some(integer) = comment.try_get_interact_integer().ok() {
                    warn!("id: {integer:?}");
                    if let Some(idx) = prev_existing_push_scopes
                        .iter()
                        .position(|id| integer & SCOPE_MASK == *id)
                    {
                        prev_existing_push_scopes.remove(idx);
                    }
                    if let Some(agent) = self.agent_mut_from_interact_integer(integer, &uri).ok() {
                        agent.cache.mut_filter_by(&role, false);
                        if integer & COMMAND_MASK == *PUSH_ID.as_ref() {
                            warn!("command is push");
                            if let Some(Token::Block(block)) = iter.next() {
                                warn!("block: {block:?}");
                                warn!("got agent, updating");
                                agent.cache.push(Message {
                                    role: role.clone(),
                                    content: block.to_owned(),
                                });
                            }
                        }
                    }
                }
            }
        }

        for scope in prev_existing_push_scopes {
            if let Some(agent) = self.agent_mut_from_interact_integer(scope, &uri).ok() {
                warn!("cleaning agent for scope: {scope}");
                agent.cache.mut_filter_by(&role, false);
            }
        }

        match self.documents.get_mut(&uri) {
            Some(tokens) => {
                *tokens = new_tokens;
            }
            None => {
                self.documents.insert(uri, new_tokens);
            }
        }

        Ok(())
    }
}

impl Clone for SharedState {
    fn clone(&self) -> Self {
        Self(Arc::clone(&self.0))
    }
}

impl SharedState {
    pub async fn init(config: Config) -> anyhow::Result<Self> {
        Ok(Self(Arc::new(RwLock::new(LspState::new(config).await?))))
    }
    pub fn get_read(&self) -> anyhow::Result<RwLockReadGuard<'_, LspState>> {
        match self.0.try_read() {
            Ok(g) => Ok(g),
            Err(e) => Err(e.into()),
        }
    }

    pub fn get_write(&mut self) -> anyhow::Result<RwLockWriteGuard<'_, LspState>> {
        match self.0.try_write() {
            Ok(g) => Ok(g),
            Err(e) => Err(e.into()),
        }
    }
}
