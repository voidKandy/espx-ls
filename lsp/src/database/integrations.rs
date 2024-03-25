use std::collections::HashMap;

use espionox::agents::memory::Message;

use crate::{
    espx_env::agents::{get_indy_agent, independent::IndyAgent},
    store::{chunks::DocumentChunk, DocUrlTup, Document},
};

use super::{DBDocument, DBDocumentChunk, DBDocumentTuple};

impl From<DBDocumentTuple> for DocUrlTup {
    fn from((dbdoc, dbdoc_chunks): DBDocumentTuple) -> Self {
        let chunks = dbdoc_chunks
            .into_iter()
            .map(|ch| DocumentChunk {
                range: ch.range,
                content: ch.content.to_owned(),
                summary: Some(ch.summary.to_owned()),
                changes: HashMap::new(),
            })
            .collect();
        DocUrlTup::new(
            dbdoc.url,
            Document {
                chunks,
                summary: Some(dbdoc.summary.to_owned()),
            },
        )
    }
}

pub(super) async fn doc_as_db_tuple(
    doc_tup: &mut DocUrlTup,
) -> Result<DBDocumentTuple, anyhow::Error> {
    let mut summarizer = get_indy_agent(IndyAgent::Summarizer).unwrap();
    let embedder = get_indy_agent(IndyAgent::Embedder).unwrap();

    summarizer.mutate_agent_cache(|c| c.push(Message::new_user(&doc_tup.1.content())));

    let summary = summarizer
        .io_completion()
        .await
        .expect("Couldn't get IO completion");
    summarizer.mutate_agent_cache(|c| {
        c.mut_filter_by(espionox::agents::memory::MessageRole::System, true)
    });

    let summary_embedding = embedder.get_embedding(&summary).await.unwrap();

    let dbdoc = DBDocument {
        url: doc_tup.0.clone(),
        summary,
        summary_embedding,
    };

    let mut dbdoc_chunks = vec![];
    for ch in doc_tup.1.chunks.iter() {
        summarizer.mutate_agent_cache(|c| c.push(Message::new_user(&ch.content)));

        let summary = summarizer
            .io_completion()
            .await
            .expect("Couldn't get IO completion");
        summarizer.mutate_agent_cache(|c| {
            c.mut_filter_by(espionox::agents::memory::MessageRole::System, true)
        });

        let summary_embedding = embedder.get_embedding(&summary).await.unwrap();

        let content_embedding = embedder.get_embedding(&ch.content).await.unwrap();

        let dbchunk = DBDocumentChunk {
            parent_url: doc_tup.0.clone(),
            content: ch.content.to_owned(),
            content_embedding,
            summary,
            summary_embedding,
            range: ch.range,
        };

        dbdoc_chunks.push(dbchunk);
    }
    Ok((dbdoc, dbdoc_chunks))
}
