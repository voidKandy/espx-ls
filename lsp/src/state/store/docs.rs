use super::{error::*, CACHE_SIZE};
use crate::util::lru::LRUCache;
use anyhow::anyhow;
use lsp_types::{TextDocumentContentChangeEvent, Uri};
use std::path::PathBuf;
use tracing::{debug, error, warn};

#[derive(Debug)]
pub struct DocLRU(pub(super) LRUCache<Uri, String>);
impl Default for DocLRU {
    fn default() -> Self {
        Self(LRUCache::new(CACHE_SIZE))
    }
}

impl DocLRU {
    #[tracing::instrument(name = "update document text from change event")]
    pub fn update_with_change(
        &mut self,
        text: &str,
        uri: Uri,
        change: &TextDocumentContentChangeEvent,
    ) -> StoreResult<()> {
        let range = change
            .range
            .ok_or(StoreError::from(anyhow!("no range in change notification")))?;

        let mut lines: Vec<String> = text.lines().map(|l| l.to_string()).collect();
        let change_lines: Vec<String> = change.text.lines().map(|l| l.to_string()).collect();

        if range.start.line as usize >= lines.len()
            || range.start.line as usize + change_lines.len() >= lines.len()
        {
            return Err(
                anyhow!("why does the change start line exceed the available lines?").into(),
            );
        }

        for (i, cl) in change_lines.into_iter().enumerate() {
            let line = &mut lines[range.start.line as usize + i];
            for (k, b) in cl.as_bytes().into_iter().enumerate() {
                let change_idx = if i == 0 {
                    range.start.character as usize + k
                } else {
                    k
                };
                if change_idx >= line.len() {
                    for _ in 0..(change_idx - line.len()) + 1 {
                        line.push(' ');
                    }
                }
                let line_as_bytes = unsafe { line.as_bytes_mut() };
                line_as_bytes[change_idx] = *b;
            }
        }

        self.0.update(uri, lines.join("\n"));
        Ok(())
    }
}
pub fn walk_dir(path: PathBuf) -> StoreResult<Vec<(PathBuf, String)>> {
    debug!("WALKING DIRECTORY: {:?}", path);
    let mut return_vec = vec![];
    match std::fs::read_dir(path.clone()) {
        Ok(read_dir) => {
            debug!("fs::read_dir returned OK");
            let filtered_entries = read_dir
                .filter_map(|res| match res {
                    Ok(r) => {
                        if r.path()
                            .as_os_str()
                            .to_string_lossy()
                            .to_string()
                            .split_once('/')
                            .unwrap()
                            .1
                            .chars()
                            .nth(0)
                            != Some('.')
                        {
                            Some(r.path())
                        } else {
                            None
                        }
                    }
                    Err(err) => {
                        error!("PROBLEM WITH READ_DIR RESPONSE: {:?}", err);
                        None
                    }
                })
                .collect::<Vec<PathBuf>>();
            debug!("Got {:?} filtered entries", filtered_entries.len());
            for entry in filtered_entries.into_iter() {
                match entry.is_dir() {
                    true => {
                        debug!("entry is directory");
                        match walk_dir(entry) {
                            Ok(mut vec) => {
                                if !vec.is_empty() {
                                    return_vec.append(&mut vec);
                                }
                            }
                            Err(err) => {
                                error!("Encountered error while walking sub-directory: {:?}", err)
                            }
                        }
                    }
                    false => {
                        debug!("entry is not directory");
                        match std::fs::read_to_string(entry.clone()) {
                            Ok(text) => return_vec.push((entry, text)),
                            Err(err) => warn!(
                                "Encountered error when reading {:?} to string: {:?}",
                                entry, err
                            ),
                        }
                    }
                }
            }
        }
        Err(err) => error!(
            "fs::read_dir encountered problem reading directory: {:?}",
            err
        ),
    }
    debug!("Returning vector of uris & texts");
    Ok(return_vec)
}
