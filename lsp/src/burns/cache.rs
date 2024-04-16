use super::error::BurnResult;
use crate::burns::{error::BurnError, Burn, InBufferBurn};
use anyhow::anyhow;
use log::{debug, info};
use lsp_types::{Position, Url};
use std::collections::HashMap;

pub type BurnMap = HashMap<u32, Vec<InBufferBurn>>;
#[derive(Debug)]
pub struct BurnCache {
    map: HashMap<Url, BurnMap>,
}

impl Default for BurnCache {
    fn default() -> Self {
        Self {
            map: HashMap::new(),
        }
    }
}

impl BurnCache {
    pub fn save_burn(&mut self, url: Url, burn: InBufferBurn) -> BurnResult<()> {
        // if let Burn::Echo(ref echo) = burn.burn {
        let line = burn.burn.range().start.line;
        match self.map.get_mut(&url) {
            Some(doc_burn_map) => match doc_burn_map.get_mut(&line) {
                Some(line_burn_vec) => {
                    line_burn_vec.push(burn);
                }
                None => {
                    doc_burn_map.insert(line, vec![burn]);
                }
            },
            None => {
                let mut burns = HashMap::new();
                burns.insert(line, vec![burn]);
                self.map.insert(url, burns);
            }
        }

        return Ok(());
        // }
        // Err(BurnError::ActionType)
    }

    pub fn get_burn_by_position(
        &mut self,
        url: &Url,
        position: Position,
    ) -> BurnResult<&mut InBufferBurn> {
        info!("LOOKING FOR BURN AT POSITION: {:?}", position);
        if let Some(map) = self.map.get_mut(url) {
            info!("MAP EXISTS: {:?}", map);
            if let Some(found_burn) = map
                .get_mut(&position.line)
                .ok_or(anyhow!("No burns on line: {}", position.line))?
                .into_iter()
                .find(|burn| {
                    debug!("ITERATING...BURN RANGE: {:?}", burn.burn.range());
                    let range = burn.burn.range();
                    position.character >= range.start.character
                        && position.character <= range.end.character
                })
            {
                info!("BURN EXISTS, RETURNING HOVER CONTENTS");
                return Ok(found_burn);
            }
        }
        info!("NO BURN FOUND");
        Err(BurnError::Undefined(anyhow!("No burns on given document")))
    }

    pub fn all_burns_on_doc(&self, url: &Url) -> Option<Vec<&InBufferBurn>> {
        if let Some(runes) = self.map.get(url) {
            info!("GOT RUNES: {:?}", runes);
            return Some(runes.values().flatten().collect());
        }
        None
    }
}
