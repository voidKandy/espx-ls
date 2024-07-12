use super::super::DatabaseStruct;
use crate::state::database::{error::DatabaseResult, Database};
use lsp_types::Uri;
use serde::{Deserialize, Serialize};
use tracing::info;

// pub type BurnMap = HashMap<u32, InBufferBurn>;
#[derive(Clone, Serialize, Deserialize, Debug, PartialEq, Eq)]
pub struct DBDocumentInfo {
    pub uri: Uri,
}

impl DatabaseStruct<Option<DBDocumentInfo>> for DBDocumentInfo {
    fn db_id() -> &'static str {
        "documents"
    }

    async fn get_all_by_uri(db: &Database, uri: &Uri) -> DatabaseResult<Option<Self>> {
        let query = format!(
            "SELECT * FROM ONLY {} where uri = $uri LIMIT 1;",
            DBDocumentInfo::db_id()
        );
        let mut response = db.client.query(query).bind(("uri", uri)).await?;
        info!("DB QUERY RESPONSE: {:?}", response);
        let doc: Option<DBDocumentInfo> = response.take(0)?;
        Ok(doc)
    }

    async fn take_all_by_uri(db: &Database, uri: &Uri) -> DatabaseResult<Option<Self>> {
        let query = format!("DELETE {} WHERE uri = $uri;", Self::db_id());

        let mut response = db.client.query(query).bind(("uri", uri)).await?;
        let doc: Option<DBDocumentInfo> = response.take(0)?;
        Ok(doc)
    }
}
