pub mod burns;
pub mod chunks;
pub mod info;
use self::burns::DBDocumentBurn;
use super::DatabaseIdentifier;
use super::{error::DatabaseResult, Database};
use crate::state::store::GlobalStore;
use anyhow::anyhow;
use chunks::{ChunkVector, DBDocumentChunk};
use info::DBDocumentInfo;
use lsp_types::Uri;
use serde::{Deserialize, Serialize};
use tracing::debug;

#[derive(Clone, Serialize, Deserialize, Debug)]
pub struct FullDBDocument {
    pub info: DBDocumentInfo,
    pub chunks: ChunkVector,
    pub burns: Vec<DBDocumentBurn>,
}

impl FullDBDocument {
    pub async fn from_state(store: &GlobalStore, uri: Uri) -> DatabaseResult<FullDBDocument> {
        let text = store
            .read_doc(&uri)
            .map_err(|err| anyhow!("Couldn't get document: {:?}", err))?;
        let chunks = ChunkVector::from_text(uri.clone(), &text)?;
        let burns = match store.burns.read_burns_on_doc(&uri) {
            Some(map) => map.iter().fold(vec![], |mut acc, (line, burn)| {
                acc.push(DBDocumentBurn::from(&uri, vec![*line], burn));
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

impl FullDBDocument {
    #[tracing::instrument(name = "Get all docs in database", skip_all)]
    pub async fn get_all_docs(db: &Database) -> DatabaseResult<Vec<FullDBDocument>> {
        let infos_query = format!("SELECT * from {}", DBDocumentInfo::db_id());
        let chunks_query = format!("SELECT * from {}", DBDocumentChunk::db_id());
        let burns_query = format!("SELECT * from {}", DBDocumentBurn::db_id());

        let mut response = db
            .client
            .query(infos_query)
            .query(chunks_query)
            .query(burns_query)
            .await?;
        debug!("Got response: {:?}", response);

        let infos: Vec<DBDocumentInfo> = response.take(0)?;
        debug!("Got INFOS: {:?}", infos);
        let mut chunks: Vec<DBDocumentChunk> = response.take(1)?;
        debug!("Got CHUNKS {:?}", chunks);
        let mut burns: Vec<DBDocumentBurn> = response.take(2)?;
        debug!("Got BURNS {:?}", burns);

        let mut result: Vec<FullDBDocument> = vec![];
        for info in infos.into_iter() {
            debug!("CHUNKS: {:?}", chunks);
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
            debug!("RESULT: {:?}", result);
        }
        Ok(result)
    }

    pub async fn get_by_uri(db: &Database, uri: &Uri) -> DatabaseResult<Option<FullDBDocument>> {
        let info_opt = DBDocumentInfo::get_by_uri(db, uri).await?;
        if let Some(info) = info_opt {
            let chunks = ChunkVector::get_by_uri(db, uri).await?;
            let burns = DBDocumentBurn::get_multiple_by_uri(db, uri).await?;
            return Ok(Some(FullDBDocument {
                info,
                chunks,
                burns,
            }));
        }
        return Ok(None);
    }

    // pub async fn update_doc_store(db: &Database, text: &str, uri: Uri) -> DatabaseResult<()> {
    //     match Self::get_by_uri(db, &uri).await? {
    //         None => {
    //             let doc = FullDBDocument::from(text.to_owned(), uri.clone())
    //                 .await
    //                 .expect("Failed to build dbdoc tuple");
    //             DBDocumentInfo::insert(db, &doc.info).await?;
    //             DBDocumentChunk::insert_multiple(db, &doc.chunks).await?;
    //         }
    //         Some(doc) => {
    //             if chunks::chunk_vec_content(&doc.chunks) != text {
    //                 DBDocumentChunk::remove_multiple_by_uri(db, &uri)
    //                     .await
    //                     .expect("Could not remove chunks");
    //                 let chunks = DBDocumentChunk::chunks_from_text(uri.clone(), &text)?;
    //                 DBDocumentChunk::insert_multiple(db, &chunks).await?;
    //             }
    //         }
    //     }
    //     Ok(())
    // }
}
