use anyhow::anyhow;

use crate::database::docs::{DBDocument, DBDocumentTuple};

use super::{error::CacheError, CacheResult, GlobalCache};

impl GlobalCache {
    pub async fn as_db_doc_tuple(&self) -> CacheResult<Vec<DBDocumentTuple>> {
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
