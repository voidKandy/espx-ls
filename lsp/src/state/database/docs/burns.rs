use super::super::{error::DatabaseResult, Database, Record};
use crate::state::{burns::BurnActivation, database::DatabaseIdentifier};
use lsp_types::Uri;
use serde::{Deserialize, Serialize};

#[derive(Clone, Serialize, Deserialize, Debug)]
pub struct DBDocumentBurn {
    pub activation: BurnActivation,
    pub uri: Uri,
    pub lines: Vec<u32>,
}

impl DatabaseIdentifier for DBDocumentBurn {
    fn db_id() -> &'static str {
        "burns"
    }
}

impl DBDocumentBurn {
    pub fn from(uri: &Uri, lines: Vec<u32>, activation: &BurnActivation) -> Self {
        Self {
            uri: uri.clone(),
            lines,
            activation: activation.clone(),
        }
    }

    pub async fn get_multiple_by_uri(db: &Database, uri: &Uri) -> DatabaseResult<Vec<Self>> {
        let query = format!("SELECT * FROM {} WHERE uri == $uri", "burns");
        let mut response = db.client.query(query).bind(("uri", uri)).await?;
        let burns: Vec<Self> = response.take(0)?;
        Ok(burns)
    }

    pub async fn insert(&self, db: &Database) -> DatabaseResult<Record> {
        let mut burn_vec = db.client.create("burns").content(self).await?;
        let r: Record = burn_vec.remove(0);
        Ok(r)
    }
}
