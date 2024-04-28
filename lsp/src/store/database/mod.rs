pub mod chunks;
pub mod docs;
pub mod error;
pub mod handle;
pub mod tests;
use crate::{burns::InBufferBurn, config::DatabaseConfig};
use anyhow::anyhow;
use chunks::*;
use docs::*;
use handle::DatabaseHandle;
use log::info;
use lsp_types::Url;
use serde::Deserialize;
use std::time::Duration;
use surrealdb::{
    engine::remote::ws::{Client, Ws},
    sql::Thing,
    Surreal,
};
use tokio::time::sleep;

use self::error::DBModelResult;

#[derive(Debug)]
pub struct Database {
    pub client: Surreal<Client>,
    handle: Option<DatabaseHandle>,
}

#[derive(Debug, Deserialize)]
pub struct Record {
    #[allow(dead_code)]
    id: Thing,
}

impl Database {
    pub async fn init(config: &DatabaseConfig) -> DBModelResult<Self> {
        let client: Surreal<Client> = Surreal::init();
        let handle = DatabaseHandle::try_init(config);

        info!("DB CLIENT AND HANDLE INITIATED, SLEEPING 300MS");
        sleep(Duration::from_millis(300)).await;

        let url = match &config.host {
            Some(host) => format!("{}:{}", host, config.port),
            None => format!("0.0.0.0:{}", config.port),
        };

        client.connect::<Ws>(url).await?;
        client
            .use_ns(config.namespace.as_str())
            .use_db(config.database.as_str())
            .await?;
        info!("DB CLIENT CONNECTED");

        Ok(Self { client, handle })
    }

    pub async fn kill_handle(&mut self) -> DBModelResult<()> {
        self.handle
            .take()
            .ok_or(anyhow!("Handle was none"))?
            .kill()
            .await?;
        Ok(())
    }

    pub async fn update_doc_store(&self, text: &str, url: &Url) -> DBModelResult<()> {
        info!("DID OPEN GOT DATABASE READ");
        match self.get_doc_tuple_by_url(&url).await? {
            None => {
                info!("DID OPEN NEEDS TO BUILD DB TUPLE");
                let tup = DBDocument::build_tuple(text.to_owned(), url.clone())
                    .await
                    .expect("Failed to build dbdoc tuple");
                info!("DID OPEN BUILT TUPLE");
                self.insert_document(&tup.0).await?;
                self.insert_chunks(&tup.1).await?;
            }
            Some((_, chunks)) => {
                info!("DID OPEN HAS TUPLE");
                if chunk_vec_content(&chunks) != text {
                    info!("DID OPEN UPDATING");
                    // THIS IS NOT A GOOD SOLUTION BECAUSE AT SOME POINT THE SUMMARY OF THE DOC
                    // ENTRY WILL DEPRECATE
                    // ALSO
                    // A PATCH WOULD BE BETTER THAN JUST DELETING AND REPLACING ALL OF THE CHUNKS
                    self.remove_chunks_by_url(&url)
                        .await
                        .expect("Could not remove chunks");
                    let chunks = DBDocumentChunk::chunks_from_text(url.clone(), &text).await?;
                    self.insert_chunks(&chunks).await?;
                }
            }
        }
        Ok(())
    }

    pub async fn get_doc_tuple_by_url(&self, url: &Url) -> DBModelResult<Option<DBDocumentTuple>> {
        info!("DB GETTING DOC TUPLE");
        let doc_opt = self.get_doc_by_url(url).await?;
        if let Some(doc) = doc_opt {
            let chunks = self.get_chunks_by_url(url).await?;
            return Ok(Some((doc, chunks)));
        }
        return Ok(None);
    }

    pub async fn insert_burn(&self, burn: &InBufferBurn) -> DBModelResult<Record> {
        let mut burn_vec = self.client.create("burns").content(burn).await?;
        let r: Record = burn_vec.remove(0);
        Ok(r)
    }

    pub async fn insert_document(&self, doc: &DBDocument) -> DBModelResult<Record> {
        let r = self
            .client
            .create((DBDocument::db_id(), doc.url.as_str()))
            .content(doc)
            .await?
            .expect("Failed to insert");
        Ok(r)
    }

    pub async fn insert_chunks(
        &self,
        chunks: &Vec<DBDocumentChunk>,
    ) -> Result<Vec<Record>, anyhow::Error> {
        let mut records = vec![];
        for chunk in chunks.iter() {
            records.append(
                &mut self
                    .client
                    .create(DBDocumentChunk::db_id())
                    .content(chunk)
                    .await?,
            )
        }
        Ok(records)
    }

    pub async fn remove_burn_by_url(&self, url: &Url) -> DBModelResult<()> {
        let query = format!("DELETE {} WHERE url = $url", "burns");
        self.client.query(query).bind(("url", url)).await?;
        Ok(())
    }

    pub async fn remove_doc_by_url(&self, url: &Url) -> DBModelResult<Option<DBDocument>> {
        Ok(self
            .client
            .delete((DBDocument::db_id(), url.as_str()))
            .await
            .expect("Failed to delete"))
    }

    pub async fn remove_chunks_by_url(&self, url: &Url) -> DBModelResult<()> {
        let query = format!(
            "DELETE {} WHERE parent_url = $url",
            DBDocumentChunk::db_id()
        );

        self.client.query(query).bind(("url", url)).await?;
        Ok(())
    }

    pub async fn get_burns_by_url(&self, url: &Url) -> DBModelResult<Vec<InBufferBurn>> {
        let query = format!("SELECT * FROM {} WHERE url == $url", "burns");

        let mut response = self.client.query(query).bind(("url", url)).await?;
        let burns: Vec<InBufferBurn> = response.take(0)?;
        Ok(burns)
    }

    pub async fn get_doc_by_url(&self, url: &Url) -> DBModelResult<Option<DBDocument>> {
        let query = format!(
            "SELECT * FROM ONLY {} where url = $url LIMIT 1",
            DBDocument::db_id()
        );

        info!("DB QUERY CONSTRUCTED");

        let mut response = self.client.query(query).bind(("url", url)).await?;
        info!("DB QUERY RESPONSE: {:?}", response);
        let doc: Option<DBDocument> = response.take(0)?;
        Ok(doc)
    }

    pub async fn get_chunks_by_url(&self, url: &Url) -> DBModelResult<ChunkVector> {
        let query = format!(
            "SELECT * FROM {} WHERE parent_url == $url",
            DBDocumentChunk::db_id()
        );

        let mut response = self.client.query(query).bind(("url", url)).await?;
        let docs: Vec<DBDocumentChunk> = response.take(0)?;
        Ok(docs)
    }

    pub async fn get_relavent_docs(
        &self,
        embedding: Vec<f32>,
        threshold: f32,
    ) -> DBModelResult<Vec<DBDocument>> {
        let query = format!("SELECT summary, url FROM documents WHERE vector::similarity::cosine(summary_embedding, $embedding) > {};", threshold );

        let mut response = self
            .client
            .query(query)
            .bind(("embedding", embedding))
            .await?;
        let docs: Vec<DBDocument> = response.take(0)?;
        Ok(docs)
    }
}
