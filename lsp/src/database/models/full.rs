use super::super::{error::DatabaseResult, Database};
use super::{thing_to_uri, DBBurn, DBBurnParams, DBChunk, DBChunkParams, DatabaseStruct, Record};
use crate::state::burns::Burn;
use lsp_types::Uri;
use serde::{Deserialize, Serialize};
use surrealdb::sql::Thing;
use tracing::debug;

#[derive(Clone, Serialize, Deserialize, Debug, PartialEq)]
pub struct DBDocument {
    pub id: Thing,
    pub uri: Uri,
    #[serde(skip)]
    pub chunks: Vec<DBChunk>,
    #[serde(skip)]
    pub burns: Vec<DBBurn>,
}

#[derive(Clone, Serialize, Debug, PartialEq)]
pub struct DBDocumentParams {
    pub uri: Uri,
    #[serde(skip)]
    pub chunks: Vec<DBChunkParams>,
    #[serde(skip)]
    pub burns: Vec<DBBurnParams>,
}

impl ToString for DBDocument {
    fn to_string(&self) -> String {
        let uri = thing_to_uri(&self.id).unwrap();
        format!(
            r#"
        [ START OF DOCUMENT: {} ]
        {}
        [ END OF DOCUMENT: {} ]
        "#,
            uri.as_str(),
            self.chunks
                .iter()
                .map(|ch| ch.to_string())
                .collect::<Vec<String>>()
                .join("\n"),
            uri.as_str(),
        )
    }
}

impl DatabaseStruct<DBDocumentParams> for DBDocument {
    fn db_id() -> &'static str {
        "documents"
    }
    fn thing(&self) -> &Thing {
        &self.id
    }

    async fn get_all(db: &Database) -> DatabaseResult<Vec<Self>> {
        let response: Vec<Record> = db.client.select(Self::db_id()).await?;
        let mut all = vec![];
        for r in response.into_iter() {
            let uri = super::thing_to_uri(&r.id)?;
            let chunks = DBChunk::get_by_field(db, "uri", &uri).await?;
            let burns = DBBurn::get_by_field(db, "uri", &uri).await?;
            all.push(Self {
                id: r.id,
                uri,
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
            let chunks = DBChunk::take_by_field(db, "uri", &uri).await?;
            let burns = DBBurn::take_by_field(db, "uri", &uri).await?;
            all.push(Self {
                id: r.id,
                uri,
                chunks,
                burns,
            })
        }
        Ok(all)
    }

    async fn create_one(
        mut params: DBDocumentParams,
        db: &Database,
    ) -> DatabaseResult<Option<Record>> {
        let chunks = params.chunks.drain(..).collect();
        let burns = params.burns.drain(..).collect();
        let mut r: Vec<Record> = db.client.create(Self::db_id()).content(params).await?;
        DBChunk::create_many(db, chunks).await?;
        DBBurn::create_many(db, burns).await?;
        Ok(r.pop())
    }

    async fn update_many(db: &Database, many: Vec<Self>) -> DatabaseResult<()> {
        for mut doc in many.into_iter() {
            let chunks = doc.chunks.drain(..).collect();
            let burns = doc.burns.drain(..).collect();
            let _: Vec<Record> = db.client.update(Self::db_id()).content(doc).await?;
            DBChunk::update_many(db, chunks).await?;
            DBBurn::update_many(db, burns).await?;
        }
        Ok(())
    }
}

impl DBDocumentParams {
    #[tracing::instrument(name = "build full db document")]
    pub fn build(text: &str, uri: Uri) -> DatabaseResult<Self> {
        let burns: Vec<DBBurnParams> =
            Burn::all_in_text(&text)
                .into_iter()
                .fold(vec![], |mut acc, b| {
                    acc.push(DBBurnParams::new(uri.clone(), b));
                    acc
                });
        debug!("burns: {:?}", burns);
        // let chunks = chunk_vec_from_text(uri.clone(), &text).unwrap();
        let chunks = DBChunkParams::from_text(uri.clone(), &text)?;

        debug!("chunks: {:?}", chunks);
        Ok(Self { uri, chunks, burns })
    }

    // fn vec_from_all(uris: Vec<Uri>, mut chunks: Vec<DBChunk>, mut burns: Vec<DBBurn>) -> Vec<Self> {
    //     let mut result: Vec<DBDocument> = vec![];
    //     for uri in uris.into_iter() {
    //         let (doc_chunks, remaining): (Vec<_>, Vec<_>) =
    //             chunks.drain(..).partition(|ch| ch.uri == uri);
    //         chunks = remaining;
    //
    //         let (doc_burns, remaining): (Vec<_>, Vec<_>) =
    //             burns.drain(..).partition(|b| b.uri == uri);
    //         burns = remaining;
    //
    //         let id = Thing {
    //             tb: Self::db_id().to_string(),
    //             id: surrealdb::sql::Id::String(uri.to_string()),
    //         };
    //         result.push(DBDocument {
    //             id,
    //             uri,
    //             chunks: doc_chunks.into(),
    //             burns: doc_burns,
    //         });
    //     }
    //     result
    // }
}
