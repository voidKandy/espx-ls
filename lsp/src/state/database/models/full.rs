use super::super::{error::DatabaseResult, Database};
use super::burns::DBDocumentBurn;
use super::chunks::{chunk_vec_from_text, ChunkVector, DBDocumentChunk};
use super::{thing_to_uri, DatabaseStruct, Record};
use anyhow::anyhow;
use lsp_types::Uri;
use serde::{Deserialize, Serialize};
use std::str::FromStr;
use surrealdb::sql::{Id, Thing};
use tracing::debug;

#[derive(Clone, Serialize, Deserialize, Debug, PartialEq)]
pub struct FullDBDocument {
    pub id: Uri,
    #[serde(skip)]
    pub chunks: ChunkVector,
    #[serde(skip)]
    pub burns: Vec<DBDocumentBurn>,
}

impl ToString for FullDBDocument {
    fn to_string(&self) -> String {
        format!(
            r#"
        [ START OF DOCUMENT: {} ]
        {}
        [ END OF DOCUMENT: {} ]
        "#,
            self.id.as_str(),
            self.chunks
                .iter()
                .map(|ch| ch.to_string())
                .collect::<Vec<String>>()
                .join("\n"),
            self.id.as_str(),
        )
    }
}

impl DatabaseStruct for FullDBDocument {
    fn db_id() -> &'static str {
        "documents"
    }
    fn thing(&self) -> Option<Thing> {
        let thing = Thing {
            tb: Self::db_id().to_owned(),
            id: Id::String(self.id.as_str().to_string()),
        };
        Some(thing)
    }

    fn add_id_to_me(&mut self, thing: Thing) {
        self.id = thing_to_uri(&thing).expect("failed to create uri from thing");
    }

    async fn get_all(db: &Database) -> DatabaseResult<Vec<Self>> {
        let response: Vec<Record> = db.client.select(Self::db_id()).await?;
        let mut all = vec![];
        for r in response.into_iter() {
            let uri = super::thing_to_uri(&r.id)?;
            let chunks = DBDocumentChunk::get_by_field(db, "parent_uri", &uri).await?;
            let burns = DBDocumentBurn::get_by_field(db, "uri", &uri).await?;
            all.push(Self {
                id: uri,
                chunks,
                burns,
            })
        }
        Ok(all)
    }

    async fn take_all(db: &Database) -> DatabaseResult<Vec<Self>> {
        let response: Vec<Record> = db.client.delete(Self::db_id()).await?;
        let mut all = vec![];
        for r in response.into_iter() {
            let uri = super::thing_to_uri(&r.id)?;
            let chunks = DBDocumentChunk::take_by_field(db, "parent_uri", &uri).await?;
            let burns = DBDocumentBurn::take_by_field(db, "uri", &uri).await?;
            all.push(Self {
                id: uri,
                chunks,
                burns,
            })
        }
        Ok(all)
    }

    async fn insert(&mut self, db: &Database) -> DatabaseResult<()> {
        let _: Option<Record> = db.client.create((Self::db_id(), self.id.as_str())).await?;
        DBDocumentChunk::insert_or_update_many(db, self.chunks.clone()).await?;
        DBDocumentBurn::insert_or_update_many(db, self.burns.clone()).await?;
        Ok(())
    }

    async fn insert_or_update_many(db: &Database, mut many: Vec<Self>) -> DatabaseResult<()> {
        let mut transaction_str = "BEGIN TRANSACTION;".to_owned();
        for doc in many.iter_mut() {
            let _: Option<Record> = db.client.create((Self::db_id(), doc.id.as_str())).await?;

            for (i, chunk) in doc.chunks.iter().enumerate() {
                match &chunk.id {
                    None => {
                        transaction_str.push_str(&format!(
                            r#"CREATE {} 
                    SET parent_uri = $parent_uri{},
                    content_embedding = $content_embedding{},
                    content = $content{},
                    range = $range{};"#,
                            DBDocumentChunk::db_id(),
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
                            DBDocumentChunk::db_id(),
                            id,
                            i,
                            i,
                            i,
                            i,
                        ));
                    }
                }
            }

            for (i, burn) in doc.burns.iter().enumerate() {
                match &burn.id {
                    None => {
                        transaction_str.push_str(&format!(
                            r#"CREATE {} 
                    SET activation = $activation{},
                    uri = $uri{},
                    lines = $lines{};"#,
                            DBDocumentBurn::db_id(),
                            i,
                            i,
                            i,
                        ));
                    }
                    Some(id) => {
                        transaction_str.push_str(&format!(
                            r#"UPDATE {}:{} 
                    SET activation = $activation{},
                    uri = $uri{},
                    lines = $lines{};"#,
                            DBDocumentBurn::db_id(),
                            id,
                            i,
                            i,
                            i
                        ));
                    }
                }
            }
        }

        transaction_str.push_str("COMMIT TRANSACTION;");
        debug!("running transaction: {}", transaction_str);
        let mut q = db.client.query(transaction_str);

        for doc in many.into_iter() {
            for (i, chunk) in doc.chunks.into_iter().enumerate() {
                let key = format!("parent_uri{}", i);
                q = q.bind((key, chunk.parent_uri));
                let key = format!("content_embedding{}", i);
                q = q.bind((key, chunk.content_embedding));
                let key = format!("content{}", i);
                q = q.bind((key, chunk.content));
                let key = format!("range{}", i);
                q = q.bind((key, chunk.range));
            }
            for (i, burn) in doc.burns.into_iter().enumerate() {
                let key = format!("activation{}", i);
                q = q.bind((key, burn.activation));
                let key = format!("uri{}", i);
                q = q.bind((key, burn.uri));
                let key = format!("lines{}", i);
                q = q.bind((key, burn.lines));
            }
        }
        let _ = q.await?;
        Ok(())
    }
}

impl FullDBDocument {
    pub fn new(text: &str, uri: Uri, burns: Vec<DBDocumentBurn>) -> DatabaseResult<FullDBDocument> {
        let chunks = chunk_vec_from_text(uri.clone(), &text)?;
        Ok(Self {
            id: uri,
            chunks,
            burns,
        })
    }

    fn vec_from_all(
        uris: Vec<Uri>,
        mut chunks: Vec<DBDocumentChunk>,
        mut burns: Vec<DBDocumentBurn>,
    ) -> Vec<Self> {
        let mut result: Vec<FullDBDocument> = vec![];
        for uri in uris.into_iter() {
            let (doc_chunks, remaining): (Vec<_>, Vec<_>) =
                chunks.drain(..).partition(|ch| ch.parent_uri == uri);
            chunks = remaining;

            let (doc_burns, remaining): (Vec<_>, Vec<_>) =
                burns.drain(..).partition(|b| b.uri == uri);
            burns = remaining;

            result.push(FullDBDocument {
                id: uri,
                chunks: doc_chunks.into(),
                burns: doc_burns,
            });
        }
        result
    }
}
