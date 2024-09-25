use crate::{embeddings, util::OneOf};
use anyhow::anyhow;
use lsp_types::Uri;
use serde::{Deserialize, Serialize};
use surrealdb::sql::Thing;
use tracing::{debug, info};
use tracing_log::log::error;
use tracing_subscriber::fmt::format::FieldFn;

use super::{DatabaseStruct, FieldQuery, QueryBuilder};

const LINES_IN_CHUNK: u32 = 20;

#[derive(Clone, Serialize, Deserialize, Debug, PartialEq)]
pub struct DBChunk {
    id: Thing,
    pub uri: Uri,
    pub content: String,
    pub content_embedding: Option<Vec<f32>>,
    pub range: (u32, u32),
}

#[derive(Clone, Serialize, Debug, PartialEq, Default)]
pub struct DBChunkParams {
    pub uri: Option<Uri>,
    pub content: Option<String>,
    pub content_embedding: Option<Vec<f32>>,
    pub range: Option<(u32, u32)>,
}

impl Into<OneOf<DBChunk, DBChunkParams>> for DBChunk {
    fn into(self) -> OneOf<DBChunk, DBChunkParams> {
        OneOf::Left(self)
    }
}

impl Into<OneOf<DBChunk, DBChunkParams>> for DBChunkParams {
    fn into(self) -> OneOf<DBChunk, DBChunkParams> {
        OneOf::Right(self)
    }
}

impl DatabaseStruct<DBChunkParams> for DBChunk {
    fn db_id() -> &'static str {
        "doc_chunks"
    }
    fn thing(&self) -> &Thing {
        &self.id
    }

    fn content(oneof: impl Into<OneOf<Self, DBChunkParams>>) -> DatabaseResult<String> {
        match Into::<OneOf<Self, DBChunkParams>>::into(oneof) {
            OneOf::Left(me) => {
                let content_embedding_string = {
                    if let Some(emb) = me.content_embedding {
                        format!("content_embedding: {},", serde_json::to_value(emb)?)
                    } else {
                        String::new()
                    }
                };

                Ok(format!(
                    r#"CONTENT {{
                    uri: {},
                    content: {},
                    range: {},
                    {}
                }}"#,
                    serde_json::to_value(me.uri)?,
                    serde_json::to_value(me.content)?,
                    serde_json::to_value(me.range)?,
                    content_embedding_string,
                ))
            }
            OneOf::Right(params) => {
                let uri_string = {
                    if let Some(uri) = params.uri {
                        format!(",uri: {}", serde_json::to_value(uri.as_str())?)
                    } else {
                        String::new()
                    }
                };

                let content_string = {
                    if let Some(content) = params.content {
                        format!(",content: {}", serde_json::to_value(content)?)
                    } else {
                        String::new()
                    }
                };

                let range_string = {
                    if let Some(range) = params.range {
                        format!(",range: {}", serde_json::to_value(range)?)
                    } else {
                        String::new()
                    }
                };

                let content_embedding_string = {
                    if let Some(emb) = params.content_embedding {
                        format!(",content_embedding: {}", serde_json::to_value(emb)?)
                    } else {
                        String::new()
                    }
                };

                Ok(format!(
                    r#"CONTENT {{ {} }}"#,
                    [
                        uri_string,
                        content_string,
                        range_string,
                        content_embedding_string
                    ]
                    .join(" ")
                    .trim()
                    .trim_start_matches(','),
                ))
            }
        }
    }

    fn create(params: DBChunkParams) -> DatabaseResult<String> {
        if params.uri.is_none() || params.content.is_none() || params.range.is_none() {
            return Err(DatabaseError::DbStruct(format!(
                "All fields need to be Some for a create statement, got: {:?} {:?} {:?}",
                params.uri, params.content, params.range
            )));
        }

        Ok(format!(
            "CREATE {} {};",
            Self::db_id(),
            Self::content(params)?
        ))
    }
}

impl DBChunkParams {
    pub fn from_text(uri: Uri, text: &str) -> anyhow::Result<Vec<Self>> {
        let mut chunks = vec![];
        let chunked_text = chunk_text(text);
        for (range, text) in chunked_text.iter() {
            info!("CHUNKED TEXT");
            let chunk = Self {
                uri: Some(uri.clone()),
                range: Some((range.0, range.1)),
                content: Some(text.to_owned()),
                ..Default::default()
            };
            chunks.push(chunk);
        }
        Ok(chunks.into())
    }
}

// used only in the any have no embeddings function
#[derive(Debug, Deserialize)]
struct ReturnUri {
    uri: Uri,
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
        // let fq = FieldQuery::new("content_embedding", Option::<Vec<f32>>::None)?;
        let query = "SELECT uri FROM doc_chunks WHERE content_embedding = NONE";
        let docs_missing_embeds: Vec<ReturnUri> = db
            .client
            .query(query)
            .await
            .unwrap()
            .take(0)
            .expect("failed to serialize");

        debug!("got {} docs missing embeddings", docs_missing_embeds.len());

        if !docs_missing_embeds.is_empty() {
            debug!(
                "have to generate embeddings for the following documents: {:?}.",
                docs_missing_embeds
                    .iter()
                    .map(|u| u.uri.as_str())
                    .collect::<Vec<&str>>(),
            );

            let mut all: Vec<DBChunk> = {
                let mut q = QueryBuilder::begin();
                for u in docs_missing_embeds {
                    q.push(&Self::select(Some(FieldQuery::new("uri", u.uri)?), None)?);
                }
                db.client
                    .query(q.end())
                    .await
                    .expect("failed to query transaction")
                    .take(0)
                    .expect("failed to serialize response")
            };

            DBChunk::embed_all(&mut all)?;
            let mut q = QueryBuilder::begin();
            for chunk in all {
                q.push(&Self::update(chunk.id.clone(), chunk)?)
            }
            db.client
                .query(q.end())
                .await
                .expect("failed to query transaction");
        }

        let query = format!("SELECT * FROM {} WHERE vector::similarity::cosine($this.content_embedding, $embedding) > {};", DBChunk::db_id(), threshold );
        let mut response = db
            .client
            .query(query)
            .bind(("embedding", embedding))
            .await?;
        let chunks: Vec<DBChunk> = response.take(0)?;
        Ok(chunks)
    }

    #[tracing::instrument(name = "embedding all document chunks")]
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
