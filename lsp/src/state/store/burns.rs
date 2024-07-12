use lsp_types::Uri;
use std::collections::HashMap;

use crate::state::burns::BurnActivation;

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

    pub fn take_burns_on_doc(&mut self, uri: &Uri) -> Option<BurnLineMap> {
        self.map.remove(uri)
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
