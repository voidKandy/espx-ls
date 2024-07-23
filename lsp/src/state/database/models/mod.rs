pub mod burns;
pub mod chunks;
pub mod full;

use std::str::FromStr;

pub use self::{burns::*, chunks::*, full::*};

use anyhow::anyhow;
use lsp_types::Uri;
use serde::{Deserialize, Serialize};
use surrealdb::sql::Thing;

use super::{error::DatabaseResult, Database, Record};

pub fn thing_to_uri(thing: &Thing) -> anyhow::Result<Uri> {
    let chars_to_remove = ['⟩', '⟨'];
    let mut sani_id = thing.id.to_string();
    for c in chars_to_remove {
        sani_id = sani_id.replace(c, "");
    }
    Uri::from_str(&sani_id)
        .map_err(|err| anyhow!("could not build uri from id: {:?}\nID: {}", err, sani_id))
}

pub trait DatabaseStruct<P>: Serialize + for<'de> Deserialize<'de> + Sized
where
    P: Serialize,
{
    fn db_id() -> &'static str;
    fn thing(&self) -> &Thing;
    async fn create_one(params: P, db: &Database) -> DatabaseResult<Option<Record>> {
        let mut r: Vec<Record> = db.client.create(Self::db_id()).content(params).await?;
        Ok(r.pop())
    }
    #[allow(unused)]
    async fn update_many(db: &Database, many: Vec<Self>) -> DatabaseResult<()> {
        Err(anyhow!("unimplemented").into())
    }
    async fn create_many(db: &Database, many: Vec<P>) -> DatabaseResult<()> {
        Err(anyhow!("unimplemented").into())
    }
    async fn get_all(db: &Database) -> DatabaseResult<Vec<Self>> {
        let response: Vec<Self> = db.client.select(Self::db_id()).await?;
        Ok(response)
    }
    async fn take_all(db: &Database) -> DatabaseResult<Vec<Self>> {
        let query = format!("DELETE {};", Self::db_id());
        let mut response = db.client.query(query).await?;
        let r = response.take(0)?;
        Ok(r)
    }

    async fn get_by_id(db: &Database, id: &str) -> DatabaseResult<Option<Self>> {
        let me: Option<Self> = db.client.select((Self::db_id(), id)).await?;
        Ok(me)
    }
    async fn take_by_id(db: &Database, id: &str) -> DatabaseResult<Option<Self>> {
        let me: Option<Self> = db.client.select((Self::db_id(), id)).await?;
        Ok(me)
    }
    async fn update_single(db: &Database, me: Self) -> DatabaseResult<Option<Self>> {
        let thing = me.thing().to_owned();
        let me: Option<Self> = db.client.update((thing.tb, thing.id)).content(me).await?;
        Ok(me)
    }

    async fn get_by_field(
        db: &Database,
        field_name: &str,
        field_val: &impl Serialize,
    ) -> DatabaseResult<Vec<Self>> {
        let query = format!(
            "SELECT * FROM {} WHERE {} = $field_val;",
            Self::db_id(),
            field_name
        );
        let mut response = db
            .client
            .query(query)
            .bind(("field_val", field_val))
            .await?;
        let burns: Vec<Self> = response.take(0)?;
        Ok(burns)
    }

    async fn take_by_field(
        db: &Database,
        field_name: &str,
        field_val: &impl Serialize,
    ) -> DatabaseResult<Vec<Self>> {
        let query = format!(
            "DELETE {} WHERE {} = $field_val;",
            Self::db_id(),
            field_name
        );
        let mut response = db
            .client
            .query(query)
            .bind(("field_val", field_val))
            .await?;
        let burns: Vec<Self> = response.take(0)?;
        Ok(burns)
    }
}
