use espionox::agents::memory::{Message, MessageRole};
use lsp_types::Url;
use serde::{Deserialize, Serialize};

use crate::espx_env::agents::{
    get_indy_agent,
    independent::{IndyAgent, SUMMARIZE_WHOLE_DOC_PROMPT},
};

use super::{chunks::ChunkVector, error::DbModelError, DBDocumentChunk};

pub type DBDocumentTuple = (DBDocument, ChunkVector);

#[derive(Clone, Serialize, Deserialize, Debug)]
pub struct DBDocument {
    pub(super) url: Url,
    pub(super) summary: String,
    pub(super) summary_embedding: Vec<f32>,
}

impl DBDocument {
    pub fn db_id() -> &'static str {
        "documents"
    }

    pub async fn build_tuple(text: String, url: Url) -> Result<DBDocumentTuple, DbModelError> {
        let chunks = DBDocumentChunk::chunks_from_text(url.clone(), &text).await?;

        let mut summarizer = get_indy_agent(IndyAgent::Summarizer)
            .ok_or(DbModelError::FailedToGetAgent(IndyAgent::Summarizer))?;
        let embedder = get_indy_agent(IndyAgent::Embedder)
            .ok_or(DbModelError::FailedToGetAgent(IndyAgent::Embedder))?;

        summarizer.mutate_agent_cache(|c| {
            c.push(Message::new_user(&format!(
                "{} [Beginning of document]{}[End of document]",
                SUMMARIZE_WHOLE_DOC_PROMPT, text
            )))
        });

        let summary = summarizer.io_completion().await?;
        summarizer.mutate_agent_cache(|c| c.mut_filter_by(MessageRole::System, true));

        let summary_embedding = embedder.get_embedding(&summary).await?;

        let doc = DBDocument {
            url,
            summary,
            summary_embedding,
        };

        Ok((doc, chunks))
    }
}
