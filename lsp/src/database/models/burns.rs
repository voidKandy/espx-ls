use super::{
    super::{error::DatabaseResult, Database},
    DBChunkParams, DatabaseStruct,
};
use crate::{
    state::{
        burns::Burn,
        database::{error::DatabaseError, Record},
    },
    util::OneOf,
};
use lsp_types::Uri;
use serde::{Deserialize, Serialize};
use surrealdb::sql::Thing;
use tracing::{debug, info, instrument};
use tracing_log::log::error;

#[derive(Clone, Serialize, Deserialize, Debug, PartialEq)]
pub struct DBBurn {
    id: Thing,
    pub burn: Burn,
    pub uri: Uri,
}

#[derive(Clone, Serialize, Debug, PartialEq, Default)]
pub struct DBBurnParams {
    pub burn: Option<Burn>,
    pub uri: Option<Uri>,
}

impl Into<OneOf<DBBurn, DBBurnParams>> for DBBurn {
    fn into(self) -> OneOf<DBBurn, DBBurnParams> {
        OneOf::Left(self)
    }
}

impl Into<OneOf<DBBurn, DBBurnParams>> for DBBurnParams {
    fn into(self) -> OneOf<DBBurn, DBBurnParams> {
        OneOf::Right(self)
    }
}

impl DatabaseStruct<DBBurnParams> for DBBurn {
    fn db_id() -> &'static str {
        "burns"
    }
    fn thing(&self) -> &Thing {
        &self.id
    }
    fn content(oneof: impl Into<crate::util::OneOf<Self, DBBurnParams>>) -> DatabaseResult<String> {
        match Into::<OneOf<Self, DBBurnParams>>::into(oneof) {
            OneOf::Left(me) => Ok(format!(
                r#"CONTENT {{
                    burn: {},
                    uri: {},
                }}"#,
                serde_json::to_value(me.burn)?,
                serde_json::to_value(me.uri.as_str())?,
            )),
            OneOf::Right(params) => {
                let burn_string = {
                    if let Some(burn) = params.burn {
                        format!(",burn: {}", serde_json::to_value(burn)?)
                    } else {
                        String::new()
                    }
                };

                let uri_string = {
                    if let Some(uri) = params.uri {
                        format!(",uri: {}", serde_json::to_value(uri.as_str())?)
                    } else {
                        String::new()
                    }
                };

                Ok(format!(
                    r#"CONTENT {{ {} }}"#,
                    [uri_string, burn_string]
                        .join(" ")
                        .trim()
                        .trim_start_matches(','),
                ))
            }
        }
    }

    fn create(params: DBBurnParams) -> DatabaseResult<String> {
        if params.uri.is_none() || params.burn.is_none() {
            return Err(DatabaseError::DbStruct(format!(
                "All fields need to be Some for a create statement, got: {:?} {:?}",
                params.uri, params.burn
            )));
        }

        Ok(format!(
            "CREATE {} {};",
            Self::db_id(),
            Self::content(params)?
        ))
    }

    fn update(
        oneof: impl Into<OneOf<Thing, super::FieldQuery>>,
        params: impl Into<OneOf<Self, DBBurnParams>>,
    ) -> DatabaseResult<String> {
        let content = match Into::<OneOf<Self, DBBurnParams>>::into(params) {
            OneOf::Right(params) => {
                if params.uri.is_none() || params.burn.is_none() {
                    return Err(DatabaseError::DbStruct(format!(
                        "All fields need to be Some for a burn update statement, got: {:?} {:?}",
                        params.uri, params.burn
                    )));
                }
                Self::content(params)
            }
            OneOf::Left(me) => Self::content(me),
        }?;
        let q = match Into::<OneOf<Thing, super::FieldQuery>>::into(oneof) {
            OneOf::Left(thing) => format!("UPDATE {}:{} {};", Self::db_id(), thing.id, content),
            OneOf::Right(fq) => format!(
                "UPDATE {} {} WHERE {} = {};",
                Self::db_id(),
                content,
                fq.name,
                fq.val
            ),
        };
        debug!("update query: {}", q);
        Ok(q)
    }
}

impl DBBurnParams {
    pub fn from_burn(burn: Burn, uri: Uri) -> Self {
        Self {
            burn: Some(burn),
            uri: Some(uri),
        }
    }
}
