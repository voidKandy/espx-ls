pub(super) mod chunks;
pub(super) mod info;
use super::error::DBModelResult;
use chunks::{ChunkVector, DBDocumentChunk};
use espionox::agents::memory::{Message, MessageRole, ToMessage};
use info::DBDocumentInfo;
use lsp_types::Url;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;

#[derive(Clone, Serialize, Deserialize, Debug)]
pub struct FullDBDocument {
    pub info: DBDocumentInfo,
    pub chunks: ChunkVector,
}

impl FullDBDocument {
    pub async fn new(text: String, url: Url) -> DBModelResult<FullDBDocument> {
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

        let info = DBDocumentInfo {
            url,
            // summary,
            // summary_embedding: embeddings.remove(0),
            burns: HashMap::new(),
        };

        Ok(Self { info, chunks })
    }
}

impl ToMessage for FullDBDocument {
    fn to_message(&self, _: MessageRole) -> Message {
        let role = MessageRole::Other {
            alias: String::from("DATABASE"),
            coerce_to: espionox::agents::memory::OtherRoleTo::System,
        };

        let content = format!(
            r#"
        [ START OF DOCUMENT: {} ]
        [ INFO ]
        {}
        
        [ CHUNKS ]
        {}
        [ END OF DOCUMENT: {} ]
        "#,
            self.info.url,
            self.info.to_string(),
            self.chunks
                .iter()
                .map(|ch| ch.to_string())
                .collect::<Vec<String>>()
                .join("\n"),
            self.info.url,
        );
        Message { role, content }
    }
}
