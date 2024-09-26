use crate::{
    agents::Agents,
    config::Config,
    database::Database,
    interact::{
        lexer::{position_in_range, Lexer, ParsedComment, Token},
        methods::{Interact, COMMAND_PUSH},
        registry::InteractRegistry,
    },
};
use lsp_types::{Position, Uri};
use std::{collections::HashMap, sync::Arc};
use tokio::sync::{RwLock, RwLockReadGuard, RwLockWriteGuard};

pub struct SharedState(Arc<RwLock<LspState>>);

#[derive(Debug)]
pub struct LspState {
    pub documents: HashMap<Uri, Vec<Token>>,
    pub database: Option<Database>,
    pub registry: InteractRegistry,
    pub agents: Option<Agents>,
}

impl LspState {
    async fn new(mut config: Config) -> anyhow::Result<Self> {
        let database = Database::init(&mut config).await.ok();

        let mut registry = InteractRegistry::default();
        if let Some(ref commands_config) = &config.commands {
            for char in commands_config.scopes.iter() {
                registry.register_scope(&char)?;
            }
        }

        Ok(Self {
            documents: HashMap::new(),
            registry,
            database,
            agents: config.model.take().and_then(|cfg| Some(Agents::from(cfg))),
        })
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

    /// returns interact and neighboring token, if the interact requires
    pub fn interact_at_position(
        &self,
        pos: &Position,
        uri: &Uri,
    ) -> Option<(&ParsedComment, Option<&Token>)> {
        let tokens = self.documents.get(uri)?;

        if let Some(idx) = tokens.iter().position(|t| {
            if let Token::Comment(parsed) = t {
                position_in_range(&parsed.range, pos)
            } else {
                false
            }
        }) {
            if let Token::Comment(comment) = &tokens[idx] {
                let mut neighbor = None;
                if let Some(next) = tokens.iter().nth(idx + 1) {
                    if let Some(interact) = comment.try_get_interact().ok() {
                        if Interact::interract_tuple(interact)
                            .is_ok_and(|(command, _)| command == COMMAND_PUSH)
                        {
                            if let Token::Block(_) = next {
                                neighbor = Some(next);
                            }
                        }
                    }
                }
                return Some((&comment, neighbor));
            }
        }

        None
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
