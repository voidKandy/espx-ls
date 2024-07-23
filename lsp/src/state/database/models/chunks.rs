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

#[derive(Clone, Serialize, Deserialize, Debug, PartialEq)]
pub struct DBChunk {
    id: Thing,
    pub uri: Uri,
    pub content: String,
    pub content_embedding: Option<Vec<f32>>,
    pub range: (u32, u32),
}

#[derive(Clone, Serialize, Debug, PartialEq)]
pub struct DBChunkParams {
    pub uri: Uri,
    pub content: String,
    pub content_embedding: Option<Vec<f32>>,
    pub range: (u32, u32),
}

impl DatabaseStruct<DBChunkParams> for DBChunk {
    fn db_id() -> &'static str {
        "doc_chunks"
    }
    fn thing(&self) -> &Thing {
        &self.id
    }
    async fn update_many(db: &Database, many: Vec<Self>) -> DatabaseResult<()> {
        let mut transaction_str = "BEGIN TRANSACTION;".to_owned();
        for (i, one) in many.iter().enumerate() {
            transaction_str.push_str(&format!(
                r#"
                    UPDATE {}:{}
                    SET uri = $uri{},
                    content_embedding = $content_embedding{},
                    content = $content{},
                    range = $range{}"#,
                Self::db_id(),
                one.id.id.to_string(),
                i,
                i,
                i,
                i,
            ));
        }
        transaction_str.push_str("COMMIT TRANSACTION;");

        if !many.is_empty() {
            let mut q = db.client.query(transaction_str);
            for (i, one) in many.into_iter().enumerate() {
                let key = format!("uri{}", i);
                q = q.bind((key, one.uri));
                let key = format!("content_embedding{}", i);
                q = q.bind((key, one.content_embedding));
                let key = format!("content{}", i);
                q = q.bind((key, one.content));
                let key = format!("range{}", i);
                q = q.bind((key, one.range));
            }
            let _ = q.await?;
        }
        Ok(())
    }

    async fn create_many(db: &Database, many: Vec<DBChunkParams>) -> DatabaseResult<()> {
        let mut transaction_str = "BEGIN TRANSACTION;".to_owned();
        for i in 0..many.len() {
            transaction_str.push_str(&format!(
                r#"
                CREATE {}
                    SET uri = $uri{},
                    content_embedding = $content_embedding{},
                    content = $content{},
                    range = $range{}"#,
                Self::db_id(),
                i,
                i,
                i,
                i,
            ));
        }
        transaction_str.push_str("COMMIT TRANSACTION;");

        if !many.is_empty() {
            let mut q = db.client.query(transaction_str);
            for (i, one) in many.into_iter().enumerate() {
                let key = format!("uri{}", i);
                q = q.bind((key, one.uri));
                let key = format!("content_embedding{}", i);
                q = q.bind((key, one.content_embedding));
                let key = format!("content{}", i);
                q = q.bind((key, one.content));
                let key = format!("range{}", i);
                q = q.bind((key, one.range));
            }
            let _ = q.await?;
        }
        Ok(())
    }
}

impl DBChunkParams {
    pub fn new(uri: Uri, starting_line: u32, ending_line: u32, content: String) -> Self {
        Self {
            content_embedding: None,
            uri,
            range: (starting_line, ending_line),
            content,
        }
    }

    pub fn from_text(uri: Uri, text: &str) -> anyhow::Result<Vec<Self>> {
        let mut chunks = vec![];
        let chunked_text = chunk_text(text);
        for (range, text) in chunked_text.iter() {
            info!("CHUNKED TEXT");
            let chunk = Self::new(uri.clone(), range.0, range.1, text.to_string());
            chunks.push(chunk);
        }
        Ok(chunks.into())
    }
}

impl DBChunk {
    pub async fn any_have_no_embeddings(db: &Database) -> DatabaseResult<bool> {
        let query = format!(
            "SELECT * FROM {} WHERE content_embedding = None;",
            DBChunk::db_id()
        );
        let mut response = db.client.query(query).await?;
        let chunks: Vec<DBChunk> = response.take(0)?;
        Ok(!chunks.is_empty())
    }

    pub async fn get_relavent(
        db: &Database,
        embedding: Vec<f32>,
        threshold: f32,
    ) -> DatabaseResult<Vec<Self>> {
        let mut missing_embeds =
            Self::get_by_field(db, "content_embedding", &Option::<Vec<f32>>::None).await?;

        if !missing_embeds.is_empty() {
            DBChunk::embed_all(&mut missing_embeds)?;
            Self::update_many(db, missing_embeds).await?;
        }

        let query = format!("SELECT * FROM {} WHERE vector::similarity::cosine(content_embedding, $embedding) > {};", DBChunk::db_id(), threshold );
        let mut response = db
            .client
            .query(query)
            .bind(("embedding", embedding))
            .await?;
        let chunks: Vec<DBChunk> = response.take(0)?;
        Ok(chunks)
    }

    pub fn embed_all(vec: &mut Vec<DBChunk>) -> anyhow::Result<()> {
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
}

impl ToString for DBChunk {
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

pub fn get_chunk_mut_by_line(vec: &mut Vec<DBChunk>, line: u32) -> Option<&mut DBChunk> {
    vec.iter_mut()
        .find(|c| c.range.1 == line || c.range.0 == line)
}

pub fn get_chunk_ref_by_line(vec: &Vec<DBChunk>, line: u32) -> Option<&DBChunk> {
    vec.iter().find(|c| c.range.1 == line || c.range.0 == line)
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
