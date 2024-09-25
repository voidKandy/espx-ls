// pub mod burns;
// pub mod chunks;
// pub mod full;

use std::str::FromStr;

use crate::util::OneOf;

use anyhow::anyhow;
use lsp_types::Uri;
use serde::{Deserialize, Serialize};
use surrealdb::sql::Thing;
use tracing::debug;

use super::{
    error::{DatabaseError, DatabaseResult},
    Database, Record,
};

pub fn thing_to_uri(thing: &Thing) -> anyhow::Result<Uri> {
    let chars_to_remove = ['⟩', '⟨'];
    let mut sani_id = thing.id.to_string();
    for c in chars_to_remove {
        sani_id = sani_id.replace(c, "");
    }
    Uri::from_str(&sani_id)
        .map_err(|err| anyhow!("could not build uri from id: {:?}\nID: {}", err, sani_id))
}

#[derive(Debug, Clone)]
pub struct FieldQuery {
    name: String,
    val: serde_json::Value,
}

impl FieldQuery {
    pub fn new(name: &str, val: impl Serialize) -> DatabaseResult<Self> {
        Ok(Self {
            name: name.to_string(),
            val: serde_json::to_value(val)?,
        })
    }
}

#[derive(Debug, Clone)]
pub struct QueryBuilder(String);
impl QueryBuilder {
    pub fn begin() -> Self {
        Self("BEGIN TRANSACTION;".to_string())
    }
    pub fn push(&mut self, query: &str) {
        self.0 = format!("{} {}", self.0, query);
    }
    #[tracing::instrument(name = "ending transaction building", skip(self))]
    pub fn end(mut self) -> String {
        self.push("COMMIT TRANSACTION;");
        debug!("TRANSACTION STRING: {}", self.0);
        self.0
    }
}

impl Into<OneOf<Thing, FieldQuery>> for Thing {
    fn into(self) -> OneOf<Thing, FieldQuery> {
        OneOf::Left(self)
    }
}

impl Into<OneOf<Thing, FieldQuery>> for FieldQuery {
    fn into(self) -> OneOf<Thing, FieldQuery> {
        OneOf::Right(self)
    }
}
/// P is a params object. Should contain options of every possible field in Self
pub trait DatabaseStruct<P>:
    Serialize + for<'de> Deserialize<'de> + Sized + Into<OneOf<Self, P>>
where
    P: Serialize + Into<OneOf<Self, P>>,
{
    /// table id of the struct
    fn db_id() -> &'static str;
    /// for easy access to ID from reference to trait object
    fn thing(&self) -> &Thing;
    fn content(oneof: impl Into<OneOf<Self, P>>) -> DatabaseResult<String>;
    fn merge(oneof: impl Into<OneOf<Self, P>>) -> DatabaseResult<String> {
        let str = Self::content(oneof)?;
        if !str.contains("CONTENT") {
            return Err(DatabaseError::Undefined(anyhow!(
                "invalid content statement: {}",
                str
            )));
        }
        Ok(format!(
            "MERGE {}",
            str.to_string().trim_start_matches("CONTENT")
        ))
    }

    fn create(params: P) -> DatabaseResult<String>;

    /// Updates by either ID (Single) or matching field value (Multiple)
    fn update(
        oneof: impl Into<OneOf<Thing, FieldQuery>>,
        params: impl Into<OneOf<Self, P>>,
    ) -> DatabaseResult<String> {
        let q = match Into::<OneOf<Thing, FieldQuery>>::into(oneof) {
            OneOf::Left(thing) => format!(
                "UPDATE {}:{} {};",
                Self::db_id(),
                thing.id,
                Self::merge(params)?
            ),
            OneOf::Right(fq) => format!(
                "UPDATE {} {} WHERE {} = {};",
                Self::db_id(),
                Self::merge(params)?,
                fq.name,
                fq.val
            ),
        };
        debug!("update query: {}", q);
        Ok(q)
    }

    /// Deletes by either ID (Single) or matching field value (Multiple)
    fn delete(oneof: impl Into<OneOf<Thing, FieldQuery>>) -> DatabaseResult<String> {
        match Into::<OneOf<Thing, FieldQuery>>::into(oneof) {
            OneOf::Left(thing) => Ok(format!("DELETE {}:{};", Self::db_id(), thing.id,)),
            OneOf::Right(fq) => Ok(format!(
                "DELETE {} WHERE {} = {};",
                Self::db_id(),
                fq.name,
                fq.val
            )),
        }
    }

    /// Selects by either ID (Single) or matching field value (Multiple)
    fn select(
        oneof: Option<impl Into<OneOf<Thing, FieldQuery>>>,
        fieldname: Option<&str>,
    ) -> DatabaseResult<String> {
        match oneof.and_then(|of| Some(Into::<OneOf<Thing, FieldQuery>>::into(of))) {
            Some(OneOf::Left(thing)) => Ok(format!(
                "SELECT {} FROM {}:{};",
                fieldname.unwrap_or("*"),
                Self::db_id(),
                thing.id,
            )),
            Some(OneOf::Right(fq)) => Ok(format!(
                "SELECT {} FROM {} WHERE {} = {};",
                fieldname.unwrap_or("*"),
                Self::db_id(),
                fq.name,
                fq.val
            )),
            None => Ok(format!(
                "SELECT {} FROM {};",
                fieldname.unwrap_or("*"),
                Self::db_id(),
            )),
        }
    }
}
