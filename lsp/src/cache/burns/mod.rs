use anyhow::anyhow;
use log::info;
use lsp_types::{HoverContents, Position, Url};
use std::{
    collections::HashMap,
    sync::{Arc, Mutex},
};

use crate::{burns::BufferBurn, cache::error::CacheError};

use super::CacheResult;

pub type BurnMap = HashMap<u32, Vec<BufferBurn>>;
#[derive(Debug)]
pub struct GlobalRunes {
    // pub listener_burns: Arc<Mutex<Vec<(Url, BufferBurn)>>>,
    map: HashMap<Url, BurnMap>,
}
impl Default for GlobalRunes {
    fn default() -> Self {
        Self {
            // listener_burns: Arc::new(Mutex::new(vec![])),
            map: HashMap::new(),
        }
    }
}

impl GlobalRunes {
    // pub fn push_listener_burns(&mut self) -> CacheResult<()> {
    //     let burns: Vec<(Url, BufferBurn)> = self
    //         .listener_burns
    //         .lock()
    //         .expect("Failed to lock mutex")
    //         .drain(..)
    //         .collect();
    //     for (url, burn) in burns.into_iter() {
    //         self.save_burn(url, burn)?;
    //     }
    //     Ok(())
    // }

    pub fn save_burn(&mut self, url: Url, burn: BufferBurn) -> CacheResult<()> {
        let line = burn.range().start.line;
        match self.map.get_mut(&url) {
            Some(doc_rune_map) => match doc_rune_map.get_mut(&line) {
                Some(line_rune_vec) => {
                    line_rune_vec.push(burn);
                }
                None => {
                    doc_rune_map.insert(line, vec![burn]);
                }
            },
            None => {
                let mut burns = HashMap::new();
                burns.insert(line, vec![burn]);
                self.map.insert(url, burns);
            }
        }

        Ok(())
    }

    pub fn get_burn_by_position(
        &self,
        url: &Url,
        position: Position,
    ) -> CacheResult<HoverContents> {
        info!("LOOKING FOR BURN AT POSITION: {:?}", position);
        if let Some(map) = self.map.get(url) {
            info!("MAP EXISTS: {:?}", map);

            if let Some(found_burn) = map
                .get(&position.line)
                .ok_or(anyhow!("No burns on line: {}", position.line))?
                .into_iter()
                .find(|burn| {
                    let range = burn.range();
                    position.character >= range.start.character
                        && position.character <= range.end.character
                })
            {
                info!("BURN EXISTS, RETURNING HOVER CONTENTS");
                return Ok(found_burn.hover_contents.clone());
            }
        }
        Err(CacheError::NotPresent)
    }

    pub fn all_burns_on_doc(&self, url: &Url) -> CacheResult<Vec<&BufferBurn>> {
        let runes = self.map.get(url).ok_or(CacheError::NotPresent)?;
        info!("GOT RUNES: {:?}", runes);
        Ok(runes.values().flatten().collect())
    }
}
