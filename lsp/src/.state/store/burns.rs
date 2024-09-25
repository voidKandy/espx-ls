use crate::state::burns::Burn;
use lsp_types::{Range, Uri};
use std::collections::HashMap;
use tracing::{debug, warn};

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
    }

    #[tracing::instrument(name = "taking burn", skip_all)]
    pub fn take_burn(&mut self, uri: &Uri, line: u32) -> Option<Burn> {
        debug!("taking burn on {}, at line: {}", uri.as_str(), line);
        let vec = self.map.get_mut(uri)?;
        if let Some(idx) = vec.into_iter().position(|b| b.activation.is_on_line(line)) {
            return Some(vec.remove(idx));
        }
        None
    }

    pub fn insert_burn(&mut self, uri: Uri, burn: Burn) {
        match self.map.get_mut(&uri) {
            Some(vec) => vec.push(burn),
            None => {
                self.map.insert(uri, vec![burn]);
            }
        }
    }
}
