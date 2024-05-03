use super::{
    chunks::ChunkVector,
    error::{DBModelError, DBModelResult},
    DBDocumentChunk,
};
use crate::{
    embeddings,
    espx_env::agents::{
        get_indy_agent,
        independent::{IndyAgent, SUMMARIZE_WHOLE_DOC_PROMPT},
    },
    store::burns::BurnMap,
};
use espionox::agents::memory::{Message, MessageRole};
use log::info;
use lsp_types::Url;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;

pub type DBDocumentTuple = (DBDocument, ChunkVector);

#[derive(Clone, Serialize, Deserialize, Debug)]
pub struct DBDocument {
    pub(super) url: Url,
    // pub(super) summary: String,
    // pub(super) summary_embedding: Vec<f32>,
    pub burns: BurnMap,
}

impl DBDocument {
    pub fn db_id() -> &'static str {
        "documents"
    }

    pub(super) async fn build_tuple(text: String, url: Url) -> DBModelResult<DBDocumentTuple> {
        let chunks = DBDocumentChunk::chunks_from_text(url.clone(), &text)?;

        // let mut summarizer = get_indy_agent(IndyAgent::Summarizer)
        //     .ok_or(DBModelError::FailedToGetAgent(IndyAgent::Summarizer))?;
        // info!("TUPLE BUILDER GOT SUMMARIZER");
        //
        // summarizer.mutate_agent_cache(|c| {
        //     c.push(Message::new_user(&format!(
        //         "{} [Beginning of document]{}[End of document]",
        //         SUMMARIZE_WHOLE_DOC_PROMPT, text
        //     )))
        // });
        //
        // let summary = summarizer.io_completion().await?;
        // info!("TUPLE BUILDER GOT SUMMARY");
        // summarizer.mutate_agent_cache(|c| c.mut_filter_by(MessageRole::System, true));
        //
        // let mut embeddings = embeddings::get_passage_embeddings(vec![&summary])?;

        // info!("TUPLE BUILDER GOT EMBEDDING");

        let doc = DBDocument {
            url,
            // summary,
            // summary_embedding: embeddings.remove(0),
            burns: HashMap::new(),
        };

        Ok((doc, chunks))
    }
}
