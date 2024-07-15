use crate::{
    embeddings,
    state::database::{error::DatabaseResult, Database},
};
use anyhow::anyhow;
use lsp_types::Uri;
use serde::{Deserialize, Serialize};
use surrealdb::sql::Thing;
use tracing::info;
use tracing_log::log::error;

use super::DatabaseStruct;

const LINES_IN_CHUNK: u32 = 20;
pub type ChunkVector = Vec<DBDocumentChunk>;

#[derive(Clone, Serialize, Deserialize, Debug, PartialEq)]
pub struct DBDocumentChunk {
    pub parent_uri: Uri,
    pub id: Option<Thing>,
    pub content: String,
    pub content_embedding: Option<Vec<f32>>,
    pub range: (u32, u32),
}

impl DatabaseStruct for DBDocumentChunk {
    fn db_id() -> &'static str {
        "doc_chunks"
    }
    fn thing(&self) -> Option<Thing> {
        self.id.as_ref().and_then(|t| Some(t.clone()))
    }
    fn add_id_to_me(&mut self, thing: Thing) {
        if self.id.is_some() {
            error!("should not be updating the id of a database struct");
        }
        self.id = Some(thing);
    }

    async fn insert_or_update_many(db: &Database, many: Vec<Self>) -> DatabaseResult<()> {
        let mut transaction_str = "BEGIN TRANSACTION;".to_owned();
        for (i, one) in many.iter().enumerate() {
            match &one.id {
                None => {
                    transaction_str.push_str(&format!(
                        r#"CREATE {} 
                    SET parent_uri = $parent_uri{},
                    content_embedding = $content_embedding{},
                    content = $content{},
                    range = $range{};"#,
                        Self::db_id(),
                        i,
                        i,
                        i,
                        i,
                    ));
                }
                Some(id) => {
                    transaction_str.push_str(&format!(
                        r#"UPDATE {}:{} 
                    SET parent_uri = $parent_uri{},
                    content_embedding = $content_embedding{},
                    content = $content{},
                    range = $range{};"#,
                        Self::db_id(),
                        id,
                        i,
                        i,
                        i,
                        i,
                    ));
                }
            }
        }
        transaction_str.push_str("COMMIT TRANSACTION;");

        let mut q = db.client.query(transaction_str);
        for (i, one) in many.into_iter().enumerate() {
            let key = format!("parent_uri{}", i);
            q = q.bind((key, one.parent_uri));
            let key = format!("content_embedding{}", i);
            q = q.bind((key, one.content_embedding));
            let key = format!("content{}", i);
            q = q.bind((key, one.content));
            let key = format!("range{}", i);
            q = q.bind((key, one.range));
        }
        let _ = q.await?;
        Ok(())
    }
}

impl DBDocumentChunk {
    pub fn new(parent_uri: Uri, starting_line: u32, ending_line: u32, content: String) -> Self {
        Self {
            id: None,
            content_embedding: None,
            parent_uri,
            range: (starting_line, ending_line),
            content,
        }
    }

    pub async fn any_have_no_embeddings(db: &Database) -> DatabaseResult<bool> {
        let query = format!(
            "SELECT * FROM {} WHERE content_embedding = None;",
            DBDocumentChunk::db_id()
        );
        let mut response = db.client.query(query).await?;
        let chunks: Vec<DBDocumentChunk> = response.take(0)?;
        Ok(!chunks.is_empty())
    }

    pub async fn get_relavent(
        db: &Database,
        embedding: Vec<f32>,
        threshold: f32,
    ) -> DatabaseResult<ChunkVector> {
        let mut missing_embeds =
            Self::get_by_field(db, "content_embedding", &Option::<Vec<f32>>::None).await?;

        if !missing_embeds.is_empty() {
            embed_all(&mut missing_embeds)?;
            Self::update_many(db, missing_embeds).await?;
        }

        let query = format!("SELECT * FROM {} WHERE vector::similarity::cosine(content_embedding, $embedding) > {};", DBDocumentChunk::db_id(), threshold );
        let mut response = db
            .client
            .query(query)
            .bind(("embedding", embedding))
            .await?;
        let chunks: Vec<DBDocumentChunk> = response.take(0)?;
        Ok(chunks)
    }
}

impl ToString for DBDocumentChunk {
    fn to_string(&self) -> String {
        format!(
            r#"
        [ START OF CHUNK IN RANGE: {:?} ]

        {}

        [ END OF CHUNK IN RANGE: {:?} ]

        "#,
            self.range, self.content, self.range
        )
    }
}

pub fn get_chunk_mut_by_line(vec: &mut ChunkVector, line: u32) -> Option<&mut DBDocumentChunk> {
    vec.iter_mut()
        .find(|c| c.range.1 == line || c.range.0 == line)
}

pub fn get_chunk_ref_by_line(vec: &ChunkVector, line: u32) -> Option<&DBDocumentChunk> {
    vec.iter().find(|c| c.range.1 == line || c.range.0 == line)
}

pub fn chunk_vec_from_text(uri: Uri, text: &str) -> anyhow::Result<ChunkVector> {
    let mut chunks = vec![];
    let chunked_text = chunk_text(text);
    for (range, text) in chunked_text.iter() {
        info!("CHUNKED TEXT");
        let chunk = DBDocumentChunk::new(uri.clone(), range.0, range.1, text.to_string());
        chunks.push(chunk);
    }
    Ok(chunks.into())
}

pub fn embed_all(vec: &mut ChunkVector) -> anyhow::Result<()> {
    let all_embeds =
        embeddings::get_passage_embeddings(vec.iter().map(|ch| ch.content.as_str()).collect())?;

    if all_embeds.len() != vec.len() {
        return Err(anyhow!(
            "expected {} embeddings, got {}",
            vec.len(),
            all_embeds.len()
        ));
    }

    let mut embiter = all_embeds.into_iter().enumerate();
    while let Some((i, emb)) = embiter.next() {
        vec[i].content_embedding = Some(emb);
    }

    Ok(())
}

/// u32 tuple end index _is_ inclusive
fn chunk_text(text: &str) -> Vec<((u32, u32), String)> {
    let mut chunks = vec![];
    let mut start = 0;
    let mut end = LINES_IN_CHUNK + start;
    let lines: Vec<&str> = text.lines().collect();
    while let Some(window) = {
        match (lines.get(start as usize), lines.get(end as usize)) {
            (Some(_), Some(_)) => Some(lines[start as usize..=end as usize].to_owned()),
            (Some(_), None) => Some(lines[start as usize..].to_owned()),
            _ => None,
        }
    } {
        chunks.push(((start, start + window.len() as u32 - 1), window.join("\n")));
        start += LINES_IN_CHUNK + chunks.len() as u32;
        end += start;
    }
    chunks
}
