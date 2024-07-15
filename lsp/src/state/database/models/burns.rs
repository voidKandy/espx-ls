use super::{
    super::{error::DatabaseResult, Database},
    DatabaseStruct,
};
use crate::state::burns::BurnActivation;
use lsp_types::Uri;
use serde::{Deserialize, Serialize};
use surrealdb::sql::Thing;
use tracing_log::log::error;

#[derive(Clone, Serialize, Deserialize, Debug, PartialEq)]
pub struct DBDocumentBurn {
    pub id: Option<Thing>,
    pub activation: BurnActivation,
    pub uri: Uri,
    pub lines: Vec<u32>,
}

impl DatabaseStruct for DBDocumentBurn {
    fn db_id() -> &'static str {
        "burns"
    }
    fn thing(&self) -> Option<Thing> {
        self.id.as_ref().and_then(|t| Some(t.clone()))
    }
    fn add_id_to_me(&mut self, thing: Thing) {
        if self.id.is_some() {
            error!("should not be updating the id of a database struct");
        }
        self.id = Some(thing);
    }
    async fn insert_or_update_many(db: &Database, many: Vec<Self>) -> DatabaseResult<()> {
        let mut transaction_str = "BEGIN TRANSACTION;".to_owned();
        for (i, one) in many.iter().enumerate() {
            match &one.id {
                None => {
                    transaction_str.push_str(&format!(
                        r#"CREATE {} 
                    SET activation = $activation{},
                    uri = $uri{},
                    lines = $lines{};"#,
                        Self::db_id(),
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
                        Self::db_id(),
                        id,
                        i,
                        i,
                        i
                    ));
                }
            }
        }
        transaction_str.push_str("COMMIT TRANSACTION;");

        let mut q = db.client.query(transaction_str);
        for (i, one) in many.into_iter().enumerate() {
            let key = format!("activation{}", i);
            q = q.bind((key, one.activation));
            let key = format!("uri{}", i);
            q = q.bind((key, one.uri));
            let key = format!("lines{}", i);
            q = q.bind((key, one.lines));
        }
        let _ = q.await?;
        Ok(())
    }
}

impl DBDocumentBurn {
    pub fn new(uri: Uri, lines: Vec<u32>, activation: BurnActivation) -> Self {
        Self {
            id: None,
            uri,
            lines,
            activation,
        }
    }
}
