use std::{
    collections::HashMap,
    sync::{Arc, Mutex, OnceLock},
};

use anyhow::anyhow;
use espionox::environment::agent::memory::{MessageRole, ToMessage};
use lsp_types::{lsif::RangeTag, Position, Range, TextDocumentContentChangeEvent, Url};

/// Hashmap of Uri keys to File content values
#[derive(Debug)]
pub struct TextStore {
    pub texts: HashMap<Url, String>,
}

#[derive(Debug)]
pub struct DocumentChange {
    pub range: Range,
    pub text: String,
}

pub struct ChangedDocument {
    pub og_text: String,
    pub changes: Vec<DocumentChange>,
}

impl TryFrom<&TextDocumentContentChangeEvent> for DocumentChange {
    type Error = anyhow::Error;
    fn try_from(value: &TextDocumentContentChangeEvent) -> Result<Self, Self::Error> {
        let range = value.range.ok_or(anyhow!("No range"))?;
        Ok(Self {
            range,
            text: value.text.to_owned(),
        })
    }
}

impl ChangedDocument {
    pub fn from_url_and_change_event(
        uri: &Url,
        change_events: &Vec<TextDocumentContentChangeEvent>,
    ) -> Option<Self> {
        let og_text = get_text_document(uri)?;
        let mut changes: Vec<DocumentChange> = vec![];
        for change in change_events.iter() {
            changes.push(change.try_into().ok()?);
        }
        println!("CHANGES: {:?}", changes);
        Some(Self { og_text, changes })
    }

    pub fn render(&self) -> String {
        let mut lines: Vec<String> = self.og_text.lines().map(|l| l.to_string()).collect();
        for change in self.changes.iter() {
            lines.iter_mut().enumerate().for_each(|(i, l)| {
                if i as u32 == change.range.start.line {
                    match change.range.start.line == change.range.end.line {
                        false => {
                            l.insert_str(
                                change.range.start.character as usize,
                                &format!("[CHANGE START]\n{}", change.text),
                            );
                        }
                        true => {
                            l.insert_str(
                                change.range.start.character as usize,
                                &format!("[CHANGE START]\n{}\n[CHANGE END]\n", change.text),
                            );
                        }
                    }
                } else if i as u32 == change.range.end.line {
                    l.insert_str(
                        (change.range.end.character - 1) as usize,
                        &format!("\n[CHANGE END]\n"),
                    );
                }
            })
        }
        lines.join("\n")
    }
}

pub static TEXT_STORE: OnceLock<Arc<Mutex<TextStore>>> = OnceLock::new();
pub static FILE_CHANGES_RECORD: OnceLock<Arc<Mutex<HashMap<Url, Vec<DocumentChange>>>>> =
    OnceLock::new();

pub fn init_text_store() {
    _ = TEXT_STORE.set(Arc::new(Mutex::new(TextStore {
        texts: HashMap::new(),
    })));

    // MAYBE MOVE CHANGES RECORD TO IT'S OWN INIT FUNCTION?
    _ = FILE_CHANGES_RECORD.set(Arc::new(Mutex::new(HashMap::new())));
}

pub fn get_text_document(uri: &Url) -> Option<String> {
    return TEXT_STORE
        .get()
        .expect("text store not initialized")
        .lock()
        .expect("text store mutex poisoned")
        .texts
        .get(uri)
        .cloned();
}

pub fn update_changes_record(
    uri: &Url,
    change: &TextDocumentContentChangeEvent,
) -> Result<(), anyhow::Error> {
    let mut record = FILE_CHANGES_RECORD.get().unwrap().lock().unwrap();
    if let Some(rec) = record.get_mut(&uri) {
        rec.push(change.try_into()?);
    } else {
        record.insert(uri.clone(), vec![change.try_into()?].into());
    }
    return Ok(());
}

#[cfg(test)]
mod tests {
    use lsp_types::{Position, TextDocumentContentChangeEvent, Url};

    use crate::text_store::{init_text_store, TEXT_STORE};

    use super::ChangedDocument;

    #[test]
    fn render_changed_doc_works() {
        init_text_store();
        let og = r#"
This is the original document

there is text here 


And text here

Text here as well
            "#;

        let uri = Url::parse("file:///tmp/foo").unwrap();
        TEXT_STORE
            .get()
            .unwrap()
            .lock()
            .unwrap()
            .texts
            .insert(uri.clone(), og.to_string());

        let changes = vec![
            TextDocumentContentChangeEvent {
                range: Some(lsp_types::Range {
                    start: Position {
                        line: 2,
                        character: 0,
                    },
                    end: Position {
                        line: 2,
                        character: 5,
                    },
                }),
                range_length: None,
                text: "abcde".to_string(),
            },
            TextDocumentContentChangeEvent {
                range: Some(lsp_types::Range {
                    start: Position {
                        line: 4,
                        character: 0,
                    },
                    end: Position {
                        line: 5,
                        character: 5,
                    },
                }),
                range_length: None,
                text: "efghijklmno\npqrst".to_string(),
            },
        ];
        let cd = ChangedDocument::from_url_and_change_event(&uri, &changes).unwrap();

        println!("{}", cd.render());
        assert!(false);
    }
}
