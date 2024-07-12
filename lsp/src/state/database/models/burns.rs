use super::super::{error::DatabaseResult, Database};
use crate::state::{burns::BurnActivation, database::DatabaseStruct};
use lsp_types::Uri;
use serde::{Deserialize, Serialize};

#[derive(Clone, Serialize, Deserialize, Debug, PartialEq)]
pub struct DBDocumentBurn {
    pub activation: BurnActivation,
    pub uri: Uri,
    pub lines: Vec<u32>,
}

impl DatabaseStruct<Vec<DBDocumentBurn>> for DBDocumentBurn {
    fn db_id() -> &'static str {
        "burns"
    }
    async fn get_all_by_uri(db: &Database, uri: &Uri) -> DatabaseResult<Vec<Self>> {
        let query = format!("SELECT * FROM {} WHERE uri == $uri;", Self::db_id());
        let mut response = db.client.query(query).bind(("uri", uri)).await?;
        let burns: Vec<Self> = response.take(0)?;
        Ok(burns)
    }
    async fn take_all_by_uri(db: &Database, uri: &Uri) -> DatabaseResult<Vec<Self>> {
        let query = format!("REMOVE * FROM {} WHERE uri == $uri;", Self::db_id());
        let mut response = db.client.query(query).bind(("uri", uri)).await?;
        let burns: Vec<Self> = response.take(0)?;
        Ok(burns)
    }
}

impl DBDocumentBurn {
    pub fn new(uri: Uri, lines: Vec<u32>, activation: BurnActivation) -> Self {
        Self {
            uri,
            lines,
            activation,
        }
    }
}
