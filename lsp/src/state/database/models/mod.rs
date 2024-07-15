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

fn thing_to_uri(thing: &Thing) -> anyhow::Result<Uri> {
    let chars_to_remove = ['⟩', '⟨'];
    let mut sani_id = thing.id.to_string();
    for c in chars_to_remove {
        sani_id = sani_id.replace(c, "");
    }
    Uri::from_str(&sani_id)
        .map_err(|err| anyhow!("could not build uri from id: {:?}\nID: {}", err, sani_id))
}

pub trait DatabaseStruct: Serialize + for<'de> Deserialize<'de> + Sized {
    fn db_id() -> &'static str;
    fn thing(&self) -> Option<Thing>;
    fn add_id_to_me(&mut self, thing: Thing);
    async fn insert(&mut self, db: &Database) -> DatabaseResult<()> {
        let mut ret = db.client.create(Self::db_id()).content(&self).await?;
        let r: Record = ret.remove(0);
        self.add_id_to_me(r.id);
        Ok(())
    }
    #[allow(unused)]
    async fn insert_or_update_many(db: &Database, many: Vec<Self>) -> DatabaseResult<()> {
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

    async fn get_by_thing(db: &Database, thing: Thing) -> DatabaseResult<Option<Self>> {
        let me: Option<Self> = db.client.select((thing.tb, thing.id)).await?;
        Ok(me)
    }
    async fn take_by_thing(db: &Database, thing: Thing) -> DatabaseResult<Option<Self>> {
        let me: Option<Self> = db.client.delete((thing.tb, thing.id)).await?;
        Ok(me)
    }
    async fn update_single(db: &Database, me: Self) -> DatabaseResult<Option<Self>> {
        match me.thing() {
            Some(thing) => {
                let thing = thing.to_owned();
                let me: Option<Self> = db.client.update((thing.tb, thing.id)).content(me).await?;
                Ok(me)
            }
            None => Err(anyhow!("no thing").into()),
        }
    }
    async fn update_many(db: &Database, many: Vec<Self>) -> DatabaseResult<()> {
        let _: Vec<Self> = db.client.update(Self::db_id()).content(many).await?;
        Ok(())
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
