use crate::burns::{
    error::{BurnError, BurnResult},
    Burn, InBufferBurn,
};
use anyhow::anyhow;
use log::info;
use lsp_types::{Position, Url};
use std::collections::HashMap;

pub type BurnMap = HashMap<u32, InBufferBurn>;
#[derive(Debug)]
pub struct BurnCache {
    pub(super) map: HashMap<Url, BurnMap>,
}

impl Default for BurnCache {
    fn default() -> Self {
        Self {
            map: HashMap::new(),
        }
    }
}

impl BurnCache {
    pub fn save_burn(&mut self, burn: InBufferBurn) -> BurnResult<()> {
        let line = burn.burn.range().start.line;
        let url = burn.url.clone();
        match self.map.get_mut(&url) {
            Some(doc_burn_map) => match doc_burn_map.get_mut(&line) {
                Some(burn_on_line) => {
                    *burn_on_line = burn;
                }
                None => {
                    doc_burn_map.insert(line, burn);
                }
            },
            None => {
                let mut burns = HashMap::new();
                burns.insert(line, burn);
                self.map.insert(url, burns);
            }
        }

        return Ok(());
    }

    pub fn get_burn_by_position(
        &mut self,
        url: &Url,
        position: Position,
    ) -> BurnResult<&mut InBufferBurn> {
        info!("LOOKING FOR BURN AT POSITION: {:?}", position);
        if let Some(map) = self.map.get_mut(url) {
            info!("MAP EXISTS: {:?}", map);
            if let Some(found_burn) = map.get_mut(&position.line) {
                info!("BURN EXISTS ON LINE, CHECKING CHAR");
                if position.character >= found_burn.burn.range().start.character
                    && position.character <= found_burn.burn.range().end.character
                {
                    info!("RETURNING BURN");
                    return Ok(found_burn);
                } else {
                    info!("BURN NOT IN CHAR RANGE");
                    return Err(BurnError::Undefined(anyhow!(
                        "Burn on document on line, but char position is wrong"
                    )));
                }
            }
        }
        info!("NO BURN FOUND");
        Err(BurnError::Undefined(anyhow!("No burns on given document")))
    }

    pub fn all_echos_on_doc(&self, url: &Url) -> Option<Vec<&InBufferBurn>> {
        if let Some(burns) = self.map.get(url) {
            let echos = burns
                .values()
                .filter(|b| {
                    if let Burn::Echo(_) = b.burn {
                        true
                    } else {
                        false
                    }
                })
                .collect();
            info!("GOT ECHOS: {:?}", echos);
            return Some(echos);
        }
        None
    }
}
