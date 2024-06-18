use crate::state::burns::BurnActivation;
use lsp_types::Uri;
use std::collections::HashMap;

pub type BurnLineMap = HashMap<u32, BurnActivation>;
#[derive(Debug)]
pub struct BurnCache {
    map: HashMap<Uri, BurnLineMap>,
}

impl Default for BurnCache {
    fn default() -> Self {
        Self {
            map: HashMap::new(),
        }
    }
}

impl BurnCache {
    pub fn read_burns_on_doc(&self, uri: &Uri) -> Option<&BurnLineMap> {
        self.map.get(uri)
    }
    pub fn read_burn(&self, uri: &Uri, line: u32) -> Option<&BurnActivation> {
        self.map.get(uri)?.get(&line)
    }

    pub fn take_burn(&mut self, uri: &Uri, line: u32) -> Option<BurnActivation> {
        self.map.get_mut(uri)?.remove(&line)
    }
    pub fn insert_burn(
        &mut self,
        uri: Uri,
        line: u32,
        burn: BurnActivation,
    ) -> Option<BurnActivation> {
        match self.map.get_mut(&uri) {
            Some(map) => map.insert(line, burn),
            None => {
                let mut map = HashMap::new();
                let ret = map.insert(line, burn);
                self.map.insert(uri, map);
                ret
            }
        }
    }
}

// impl BurnCache {
//     pub fn update_echos_from_change_event(
//         &mut self,
//         change: &TextDocumentContentChangeEvent,
//         uri: Uri,
//     ) -> StoreResult<()> {
//         let mut positions_to_remove = vec![];
//         if let Some(mut all_echos) = self.all_echos_on_doc(&uri) {
//             if let Some(range) = change.range {
//                 let mut all_echos_in_change = all_echos.iter_mut().fold(vec![], |mut acc, e| {
//                     if range.start.line >= e.range.start.line && range.end.line <= e.range.end.line
//                     {
//                         acc.push(e);
//                     }
//
//                     acc
//                 });
//
//                 all_echos_in_change.sort_by(|a, b| a.range.start.line.cmp(&b.range.start.line));
//                 for echo in all_echos_in_change {
//                     if change.text.contains(&echo.content) {
//                         let start_char = change
//                             .text
//                             .find(&echo.content)
//                             .expect("should have found echo pattern");
//                         let start_line = change.text[..start_char]
//                             .chars()
//                             .filter(|c| *c == '\n')
//                             .count();
//                         echo.range = Range {
//                             start: Position {
//                                 line: start_line as u32 + range.start.line,
//                                 character: start_char as u32 + range.start.character,
//                             },
//                             end: Position {
//                                 line: start_line as u32 + range.start.line,
//                                 character: start_char as u32
//                                     + range.start.character
//                                     + echo.content.len() as u32,
//                             },
//                         }
//                     } else {
//                         positions_to_remove.push(range.start);
//                     }
//                 }
//             }
//         }
//         for pos in positions_to_remove {
//             self.remove_echo_burn_by_position(&uri, pos)
//         }
//         Ok(())
//     }
//
//     pub fn save_burn(&mut self, burn: BurnActivation) -> StoreResult<()> {
//         let uri = burn.uri.clone();
//         match self.map.get_mut(&uri) {
//             Some(doc_burns) => doc_burns.push(burn),
//             None => {
//                 let mut burns = Vec::new();
//                 burns.push(burn);
//                 self.map.insert(uri, burns);
//             }
//         }
//
//         return Ok(());
//     }
//
//     pub fn remove_echo_burn_by_position(&mut self, uri: &Uri, position: Position) {
//         if let Some(mut all_echos) = self.all_echos_on_doc(&uri) {
//             if let Some(idx) = all_echos.iter().position(|e| {
//                 e.range.start.line == position.line && e.range.start.character == position.character
//                     || e.range.end.character == position.line
//                         && e.range.end.character == position.character
//             }) {
//                 all_echos.remove(idx);
//             }
//         }
//     }
//
//     pub fn get_burn_by_position(
//         &mut self,
//         uri: &Uri,
//         position: Position,
//     ) -> StoreResult<&mut BurnActivation> {
//         if let Some(burns) = self.map.get_mut(uri) {
//             if let Some(found_burn) = burns.iter_mut().find(|b| {
//                 let burn_range = b.burn.range();
//                 burn_range.start.line == position.line
//                     && burn_range.start.character == position.character
//                     || burn_range.end.character == position.line
//                         && burn_range.end.character == position.character
//             }) {
//                 info!("BURN EXISTS ON LINE, CHECKING CHAR");
//                 if position.character >= found_burn.burn.range().start.character
//                     && position.character <= found_burn.burn.range().end.character
//                 {
//                     info!("RETURNING BURN");
//                     return Ok(found_burn);
//                 } else {
//                     info!("BURN NOT IN CHAR RANGE");
//                     return Err(BurnError::Undefined(anyhow!(
//                         "Burn on document on line, but char position is wrong"
//                     )));
//                 }
//             }
//         }
//         info!("NO BURN FOUND");
//         Err(anyhow!("No burns on given document"))
//     }
//
//     pub fn all_in_buffer_burns_on_doc<F>(&self, uri: &Uri, pred_fn: F) -> Option<Vec<&InBufferBurn>>
//     where
//         F: Fn(&InBufferBurn) -> bool,
//     {
//         if let Some(burns) = self.map.get(uri) {
//             let echos = burns.into_iter().filter(|b| pred_fn(b)).collect();
//             debug!("GOT ECHOS: {:?}", echos);
//             return Some(echos);
//         }
//         None
//     }
//
//     pub fn all_echos_on_doc(&mut self, uri: &Uri) -> Option<Vec<&mut EchoBurn>> {
//         if let Some(burns) = self.map.get_mut(uri) {
//             let echos = burns
//                 .iter_mut()
//                 .filter_map(|b| {
//                     if let Burn::Echo(ref mut e) = b.burn {
//                         Some(e)
//                     } else {
//                         None
//                     }
//                 })
//                 .collect();
//             info!("GOT ECHOS: {:?}", echos);
//             return Some(echos);
//         }
//         None
//     }
// }
