use super::super::DatabaseIdentifier;
use crate::state::database::{error::DatabaseResult, Database, Record};
use lsp_types::Uri;
use serde::{Deserialize, Serialize};
use tracing::info;

// pub type BurnMap = HashMap<u32, InBufferBurn>;
#[derive(Clone, Serialize, Deserialize, Debug)]
pub struct DBDocumentInfo {
    pub uri: Uri,
}

impl DatabaseIdentifier for DBDocumentInfo {
    fn db_id() -> &'static str {
        "documents"
    }
}

impl DBDocumentInfo {
    pub async fn insert(&self, db: &Database) -> DatabaseResult<Record> {
        let r = db
            .client
            .create((DBDocumentInfo::db_id(), self.uri.as_str()))
            .content(self)
            .await?
            .expect("Failed to insert");
        Ok(r)
    }

    pub async fn get_by_uri(db: &Database, uri: &Uri) -> DatabaseResult<Option<DBDocumentInfo>> {
        let query = format!(
            "SELECT * FROM ONLY {} where uri = $uri LIMIT 1",
            DBDocumentInfo::db_id()
        );
        let mut response = db.client.query(query).bind(("uri", uri)).await?;
        info!("DB QUERY RESPONSE: {:?}", response);
        let doc: Option<DBDocumentInfo> = response.take(0)?;
        Ok(doc)
    }

    pub async fn remove_doc_by_uri(
        db: &Database,
        uri: &Uri,
    ) -> DatabaseResult<Option<DBDocumentInfo>> {
        Ok(db
            .client
            .delete((DBDocumentInfo::db_id(), uri.as_str()))
            .await
            .expect("Failed to delete"))
    }
}
