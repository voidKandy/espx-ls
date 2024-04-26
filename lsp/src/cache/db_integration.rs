use std::sync::RwLockWriteGuard;

use anyhow::anyhow;

use crate::{
    burns::InBufferBurn,
    database::{
        docs::{DBDocument, DBDocumentTuple},
        Database,
    },
};

use super::{
    error::{CacheError, CacheResult},
    GlobalCache,
};

impl GlobalCache {
    pub async fn update_db_from_cache(
        &self,
        database_write_lock: RwLockWriteGuard<'_, Database>,
    ) -> CacheResult<()> {
        let doc_tup = self.as_db_doc_tuple().await?;
        for (doc, chunks) in doc_tup.iter() {
            database_write_lock.insert_document(doc).await?;
            database_write_lock.insert_chunks(chunks).await?;
        }
        for burn in self
            .burns
            .map
            .values()
            .map(|b_map| b_map.values().collect::<Vec<&InBufferBurn>>())
            .flatten()
            .collect::<Vec<&InBufferBurn>>()
            .iter()
        {
            database_write_lock.insert_burn(burn).await?;
        }
        Ok(())
    }

    async fn as_db_doc_tuple(&self) -> CacheResult<Vec<DBDocumentTuple>> {
        let mut all_tups = vec![];
        for (url, text) in &self.lru.docs {
            let (mut doc, chunks) =
                DBDocument::build_tuple(text, url.clone())
                    .await
                    .map_err(|err| {
                        CacheError::Undefined(anyhow!("Error building DB tuple: {:?}", err))
                    })?;
            if let Some(burns) = self.burns.map.get(&url) {
                doc.burns = burns.clone();
            }
            all_tups.push((doc, chunks));
        }
        Ok(all_tups)
    }
}
