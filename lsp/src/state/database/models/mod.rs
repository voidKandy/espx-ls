pub mod burns;
pub mod chunks;
pub mod info;
use self::burns::DBDocumentBurn;
use super::DatabaseStruct;
use super::{error::DatabaseResult, Database};
use crate::state::store::GlobalStore;
use anyhow::anyhow;
use chunks::{ChunkVector, DBDocumentChunk};
use info::DBDocumentInfo;
use lsp_types::Uri;
use serde::{Deserialize, Serialize};
use tracing::debug;

#[derive(Clone, Serialize, Deserialize, Debug, PartialEq)]
pub struct FullDBDocument {
    pub info: DBDocumentInfo,
    pub chunks: ChunkVector,
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
            self.info.uri.as_str(),
            self.chunks
                .as_ref()
                .iter()
                .map(|ch| ch.to_string())
                .collect::<Vec<String>>()
                .join("\n"),
            self.info.uri.as_str(),
        )
    }
}

impl DatabaseStruct<Vec<FullDBDocument>> for FullDBDocument {
    fn db_id() -> &'static str {
        panic!("FullDBDocument should not call this method")
    }
    async fn insert(db: &Database, me: Self) -> DatabaseResult<super::Record> {
        let ret = DBDocumentInfo::insert(db, me.info).await?;
        for chunk in Into::<Vec<DBDocumentChunk>>::into(me.chunks).into_iter() {
            DBDocumentChunk::insert(db, chunk).await?;
        }
        for burn in me.burns.into_iter() {
            DBDocumentBurn::insert(db, burn).await?;
        }
        Ok(ret)
    }

    async fn get_all(db: &Database) -> DatabaseResult<Vec<Self>> {
        let infos_query = format!("SELECT * FROM {};", DBDocumentInfo::db_id());
        let chunks_query = format!("SELECT * FROM {};", DBDocumentChunk::db_id());
        let burns_query = format!("SELECT * FROM {};", DBDocumentBurn::db_id());

        let mut response = db
            .client
            .query(infos_query)
            .query(chunks_query)
            .query(burns_query)
            .await?;
        let infos: Vec<DBDocumentInfo> = response.take(0)?;
        let chunks: Vec<DBDocumentChunk> = response.take(1)?;
        let burns: Vec<DBDocumentBurn> = response.take(2)?;
        Ok(Self::vec_from_all(infos, chunks, burns))
    }

    async fn take_all(db: &Database) -> DatabaseResult<Vec<Self>> {
        let infos_query = format!("DELETE {};", DBDocumentInfo::db_id());
        let chunks_query = format!("DELETE {};", DBDocumentChunk::db_id());
        let burns_query = format!("DELETE {};", DBDocumentBurn::db_id());

        let mut response = db
            .client
            .query(infos_query)
            .query(chunks_query)
            .query(burns_query)
            .await?;
        let infos: Vec<DBDocumentInfo> = response.take(0)?;
        let chunks: Vec<DBDocumentChunk> = response.take(1)?;
        let burns: Vec<DBDocumentBurn> = response.take(2)?;
        Ok(Self::vec_from_all(infos, chunks, burns))
    }

    async fn take_all_by_uri(db: &Database, uri: &Uri) -> DatabaseResult<Vec<FullDBDocument>> {
        let infos_query = format!(
            "REMOVE * FROM {} WHERE uri = $uri;",
            DBDocumentInfo::db_id()
        );
        let chunks_query = format!(
            "REMOVE * FROM {} WHERE parent_uri = $uri;",
            DBDocumentChunk::db_id()
        );
        let burns_query = format!(
            "REMOVE * FROM {} WHERE uri = $uri;",
            DBDocumentBurn::db_id()
        );

        let mut response = db
            .client
            .query(infos_query)
            .bind(&uri)
            .query(chunks_query)
            .bind(&uri)
            .query(burns_query)
            .bind(&uri)
            .await?;
        let infos: Vec<DBDocumentInfo> = response.take(0)?;
        let chunks: Vec<DBDocumentChunk> = response.take(1)?;
        let burns: Vec<DBDocumentBurn> = response.take(2)?;
        Ok(Self::vec_from_all(infos, chunks, burns))
    }

    async fn get_all_by_uri(db: &Database, uri: &Uri) -> DatabaseResult<Vec<FullDBDocument>> {
        let infos_query = format!(
            "SELECT * FROM {} WHERE uri = $uri;",
            DBDocumentInfo::db_id()
        );
        let chunks_query = format!(
            "SELECT * FROM {} WHERE parent_uri = $uri;",
            DBDocumentChunk::db_id()
        );
        let burns_query = format!(
            "SELECT * FROM {} WHERE uri = $uri;",
            DBDocumentBurn::db_id()
        );

        let mut response = db
            .client
            .query(infos_query)
            .bind(&uri)
            .query(chunks_query)
            .bind(&uri)
            .query(burns_query)
            .bind(&uri)
            .await?;
        let infos: Vec<DBDocumentInfo> = response.take(0)?;
        let chunks: Vec<DBDocumentChunk> = response.take(1)?;
        let burns: Vec<DBDocumentBurn> = response.take(2)?;
        Ok(Self::vec_from_all(infos, chunks, burns))
    }
}

impl FullDBDocument {
    pub async fn from_state(store: &GlobalStore, uri: Uri) -> DatabaseResult<FullDBDocument> {
        let text = store
            .read_doc(&uri)
            .map_err(|err| anyhow!("Couldn't get document: {:?}", err))?;
        let chunks = ChunkVector::from_text(uri.clone(), &text)?;
        let burns = match store.burns.read_burns_on_doc(&uri) {
            Some(map) => map.into_iter().fold(vec![], |mut acc, (line, burn)| {
                acc.push(DBDocumentBurn::new(uri.clone(), vec![*line], burn.clone()));
                acc
            }),
            None => Vec::new(),
        };
        let info = DBDocumentInfo { uri };
        Ok(Self {
            info,
            chunks,
            burns,
        })
    }

    fn vec_from_all(
        infos: Vec<DBDocumentInfo>,
        mut chunks: Vec<DBDocumentChunk>,
        mut burns: Vec<DBDocumentBurn>,
    ) -> Vec<Self> {
        let mut result: Vec<FullDBDocument> = vec![];
        for info in infos.into_iter() {
            let (doc_chunks, remaining): (Vec<_>, Vec<_>) =
                chunks.drain(..).partition(|ch| ch.parent_uri == info.uri);
            chunks = remaining;

            let (doc_burns, remaining): (Vec<_>, Vec<_>) =
                burns.drain(..).partition(|b| b.uri == info.uri);
            burns = remaining;

            result.push(FullDBDocument {
                info,
                chunks: doc_chunks.into(),
                burns: doc_burns,
            });
        }
        result
    }
}
