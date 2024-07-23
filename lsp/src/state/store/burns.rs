use anyhow::anyhow;
use lsp_types::{Range, TextDocumentContentChangeEvent, Uri};
use std::collections::HashMap;
use tracing::{debug, warn};

use crate::state::{
    burns::{
        error::{BurnError, BurnResult},
        Activation, Burn,
    },
    database::models::burns,
};

#[derive(Debug)]
pub struct BurnCache {
    map: HashMap<Uri, Vec<Burn>>,
}

impl Default for BurnCache {
    fn default() -> Self {
        Self {
            map: HashMap::new(),
        }
    }
}

impl BurnCache {
    pub fn read_burns_on_doc(&self, uri: &Uri) -> Option<&Vec<Burn>> {
        self.map.get(uri)
    }

    pub fn read_burn(&self, uri: &Uri, line: u32) -> Option<&Burn> {
        self.map
            .get(uri)?
            .iter()
            .find(|b| b.activation.is_on_line(line))
    }

    pub fn take_burns_on_doc(&mut self, uri: &Uri) -> Option<Vec<Burn>> {
        self.map.remove(uri)
    }

    pub fn take_burns_in_range(&mut self, uri: &Uri, range: Range) -> Option<Vec<Burn>> {
        if let Some(vec) = self.map.remove(uri) {
            let (matching, not): (Vec<_>, Vec<_>) =
                vec.into_iter().partition(|b| b.activation.overlaps(&range));
            self.map.insert(uri.clone(), not.to_vec());
            return Some(matching.to_vec());
        }
        None

        // self.map.remove(uri)
    }

    pub fn take_burn(&mut self, uri: &Uri, line: u32) -> Option<Burn> {
        let vec = self.map.get_mut(uri)?;
        if let Some(idx) = vec.into_iter().position(|b| b.activation.is_on_line(line)) {
            return Some(vec.remove(idx));
        }
        None
    }

    // pub fn update_with_change(
    //     &mut self,
    //     uri: &Uri,
    //     change: &TextDocumentContentChangeEvent,
    // ) -> BurnResult<()> {
    //     let range = change.range.ok_or(anyhow!("no range in update event"))?;
    //
    //     if let Some(vec) = self.map.remove(&uri) {
    //         let mut burns_in_range = vec![];
    //         for b in vec.into_iter() {
    //             if b.is_in_range(&range) {
    //                 burns_in_range.push(b);
    //             } else {
    //                 self.insert_burn(uri.clone(), b);
    //             }
    //         }
    //
    //         let new_burns = Burn::all_in_text(&change.text);
    //
    //         match burns_in_range.len().cmp(&new_burns.len()) {
    //             std::cmp::Ordering::Less => {
    //                 warn!("less burns in range than in new text, burn hover contents will be lost");
    //                 for burn in new_burns {
    //                     self.insert_burn(uri.clone(), burn);
    //                 }
    //             }
    //             std::cmp::Ordering::Greater => {
    //                 warn!("more burns in range than in new text, burn hover contents will be lost");
    //                 for burn in new_burns {
    //                     self.insert_burn(uri.clone(), burn);
    //                 }
    //             }
    //             std::cmp::Ordering::Equal => {
    //                 let mut iterator = burns_in_range.into_iter();
    //                 while let Some(mut burn) = iterator.next() {
    //                     if let Some(closest_burn) =
    //                         new_burns.iter().fold(Option::<&Burn>::None, |mut opt, ib| {
    //                             match opt {
    //                                 None => opt = Some(ib),
    //                                 Some(ref b) => {
    //                                     if let Some(min) = b.lines().iter().min() {
    //                                         if let Some(omin) = ib.lines().iter().min() {
    //                                             if min > omin {
    //                                                 opt = Some(ib);
    //                                             }
    //                                         }
    //                                     }
    //                                 }
    //                             }
    //                             opt
    //                         })
    //                     {
    //                         if burn.matches_variant(&closest_burn) {
    //                             burn.activation = match &closest_burn.activation {
    //                                 Activation::Single(a) => Activation::Single(a.to_owned()),
    //                                 Activation::Multi(a) => Activation::Multi(a.to_owned()),
    //                             };
    //                             self.insert_burn(uri.clone(), burn)
    //                         }
    //                     }
    //                 }
    //             }
    //         }
    //     } else {
    //         debug!("No burns in change range");
    //     }
    //     Ok(())
    // }

    pub fn insert_burn(&mut self, uri: Uri, burn: Burn) {
        match self.map.get_mut(&uri) {
            Some(vec) => vec.push(burn),
            None => {
                self.map.insert(uri, vec![burn]);
            }
        }
    }
}
