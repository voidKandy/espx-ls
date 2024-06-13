use crate::state::burns::{
    echos::EchoBurn,
    error::{BurnError, BurnResult},
    Burn, InBufferBurn,
};
use anyhow::anyhow;
use lsp_types::{Position, Range, TextDocumentContentChangeEvent, Uri};
use std::collections::HashMap;
use tracing::debug;
use tracing::info;

// pub type BurnMap = HashMap<u32, InBufferBurn>;
#[derive(Debug)]
pub struct BurnCache {
    pub(super) map: HashMap<Uri, Vec<InBufferBurn>>,
}

impl Default for BurnCache {
    fn default() -> Self {
        Self {
            map: HashMap::new(),
        }
    }
}

impl BurnCache {
    pub fn update_echos_from_change_event(
        &mut self,
        change: &TextDocumentContentChangeEvent,
        url: Uri,
    ) -> BurnResult<()> {
        let mut positions_to_remove = vec![];
        if let Some(mut all_echos) = self.all_echos_on_doc(&url) {
            if let Some(range) = change.range {
                let mut all_echos_in_change = all_echos.iter_mut().fold(vec![], |mut acc, e| {
                    if range.start.line >= e.range.start.line && range.end.line <= e.range.end.line
                    {
                        acc.push(e);
                    }

                    acc
                });

                all_echos_in_change.sort_by(|a, b| a.range.start.line.cmp(&b.range.start.line));
                for echo in all_echos_in_change {
                    if change.text.contains(&echo.content) {
                        let start_char = change
                            .text
                            .find(&echo.content)
                            .expect("should have found echo pattern");
                        let start_line = change.text[..start_char]
                            .chars()
                            .filter(|c| *c == '\n')
                            .count();
                        echo.range = Range {
                            start: Position {
                                line: start_line as u32 + range.start.line,
                                character: start_char as u32 + range.start.character,
                            },
                            end: Position {
                                line: start_line as u32 + range.start.line,
                                character: start_char as u32
                                    + range.start.character
                                    + echo.content.len() as u32,
                            },
                        }
                    } else {
                        positions_to_remove.push(range.start);
                    }
                }
            }
        }
        for pos in positions_to_remove {
            self.remove_echo_burn_by_position(&url, pos)
        }
        Ok(())
    }

    pub fn save_burn(&mut self, burn: InBufferBurn) -> BurnResult<()> {
        let url = burn.url.clone();
        match self.map.get_mut(&url) {
            Some(doc_burns) => doc_burns.push(burn),
            None => {
                let mut burns = Vec::new();
                burns.push(burn);
                self.map.insert(url, burns);
            }
        }

        return Ok(());
    }

    pub fn remove_echo_burn_by_position(&mut self, url: &Uri, position: Position) {
        if let Some(mut all_echos) = self.all_echos_on_doc(&url) {
            if let Some(idx) = all_echos.iter().position(|e| {
                e.range.start.line == position.line && e.range.start.character == position.character
                    || e.range.end.character == position.line
                        && e.range.end.character == position.character
            }) {
                all_echos.remove(idx);
            }
        }
    }

    pub fn get_burn_by_position(
        &mut self,
        url: &Uri,
        position: Position,
    ) -> BurnResult<&mut InBufferBurn> {
        if let Some(burns) = self.map.get_mut(url) {
            if let Some(found_burn) = burns.iter_mut().find(|b| {
                let burn_range = b.burn.range();
                burn_range.start.line == position.line
                    && burn_range.start.character == position.character
                    || burn_range.end.character == position.line
                        && burn_range.end.character == position.character
            }) {
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

    pub fn all_in_buffer_burns_on_doc<F>(&self, url: &Uri, pred_fn: F) -> Option<Vec<&InBufferBurn>>
    where
        F: Fn(&InBufferBurn) -> bool,
    {
        if let Some(burns) = self.map.get(url) {
            let echos = burns.into_iter().filter(|b| pred_fn(b)).collect();
            debug!("GOT ECHOS: {:?}", echos);
            return Some(echos);
        }
        None
    }

    pub fn all_echos_on_doc(&mut self, url: &Uri) -> Option<Vec<&mut EchoBurn>> {
        if let Some(burns) = self.map.get_mut(url) {
            let echos = burns
                .iter_mut()
                .filter_map(|b| {
                    if let Burn::Echo(ref mut e) = b.burn {
                        Some(e)
                    } else {
                        None
                    }
                })
                .collect();
            info!("GOT ECHOS: {:?}", echos);
            return Some(echos);
        }
        None
    }
}
