use super::{
    super::{error::DatabaseResult, Database},
    DatabaseStruct,
};
use crate::state::{burns::Burn, database::Record};
use lsp_types::Uri;
use serde::{Deserialize, Serialize};
use surrealdb::sql::Thing;
use tracing::{info, instrument};
use tracing_log::log::error;

#[derive(Clone, Serialize, Deserialize, Debug, PartialEq)]
pub struct DBBurn {
    pub id: Thing,
    pub burn: Burn,
    pub uri: Uri,
}

#[derive(Clone, Serialize, Debug, PartialEq)]
pub struct DBBurnParams {
    pub burn: Burn,
    pub uri: Uri,
}

impl DatabaseStruct<DBBurnParams> for DBBurn {
    fn db_id() -> &'static str {
        "burns"
    }
    fn thing(&self) -> &Thing {
        &self.id
    }

    async fn update_many(db: &Database, many: Vec<Self>) -> DatabaseResult<()> {
        let mut transaction_str = "BEGIN TRANSACTION;".to_owned();
        for (i, one) in many.iter().enumerate() {
            transaction_str.push_str(&format!(
                r#"
                UPDATE {}:{}
                    SET uri = $uri{},
                    burn = $burn{}"#,
                Self::db_id(),
                one.id.id.to_string(),
                i,
                i,
            ));
        }
        transaction_str.push_str("COMMIT TRANSACTION;");
        info!("full transaction: {}", transaction_str);
        if !many.is_empty() {
            let mut q = db.client.query(transaction_str);
            for (i, one) in many.into_iter().enumerate() {
                let key = format!("burn{}", i);
                q = q.bind((key, one.burn));
                let key = format!("uri{}", i);
                q = q.bind((key, one.uri));
            }
            let _ = q.await?;
        }
        Ok(())
    }

    async fn create_many(db: &Database, many: Vec<DBBurnParams>) -> DatabaseResult<()> {
        let mut transaction_str = "BEGIN TRANSACTION;".to_owned();
        for i in 0..many.len() {
            transaction_str.push_str(&format!(
                r#"
               CREATE {}
                    SET uri = $uri{},
                    burn = $burn{}"#,
                Self::db_id(),
                i,
                i,
            ));
        }
        transaction_str.push_str("COMMIT TRANSACTION;");
        info!("full transaction: {}", transaction_str);
        if !many.is_empty() {
            let mut q = db.client.query(transaction_str);
            for (i, one) in many.into_iter().enumerate() {
                let key = format!("burn{}", i);
                q = q.bind((key, one.burn));
                let key = format!("uri{}", i);
                q = q.bind((key, one.uri));
            }
            let _ = q.await?;
        }
        Ok(())
    }
}

impl DBBurnParams {
    pub fn new(uri: Uri, burn: Burn) -> Self {
        Self { uri, burn }
    }
}
