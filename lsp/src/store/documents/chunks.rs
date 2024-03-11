use std::collections::HashMap;

use espionox::agents::{language_models::embed, memory::embeddings::EmbeddingVector};

use crate::espx_env::{summarize, SUMMARIZE_DOC_CHUNK_PROMPT};
use serde::{Deserialize, Serialize};

const LINES_IN_CHUNK: usize = 20;

#[derive(Clone)]
pub struct DocumentChunk {
    pub range: (usize, usize),
    pub changes: HashMap<usize, HashMap<usize, char>>,
    pub content: String,
    // Summary is only generated when stored in LTM
    pub summary: Option<String>,
}

impl super::Summarizable for DocumentChunk {
    async fn get_summary(&mut self) -> Result<(), anyhow::Error> {
        if let None = self.summary {
            summarize(Some(SUMMARIZE_DOC_CHUNK_PROMPT), &self.content).await?;
        }
        Ok(())
    }
}

impl std::fmt::Debug for DocumentChunk {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(
            f,
            "Chunk: \n   range: {:?}\n   content : {}\n   summary: {:?}\n   changes: {:?}\n",
            self.range,
            // self.content_embedding.is_some(),
            self.content,
            // self.summary_embedding.is_some(),
            self.summary,
            self.changes
        )
    }
}
//
// #[derive(Debug, Clone)]
// pub struct DocumentChange {
//     pub idx: usize,
//     pub char: char,
// }
//
// impl DocumentChunkBuilder {
//     pub async fn build(self) -> Result<DocumentChunk, anyhow::Error> {
//         // let summary = summarize(Some(SUMMARIZE_DOC_CHUNK_PROMPT), &self.content)
//         //     .await
//         //     .unwrap();
//
//         // let content_embedding = match self.content.is_empty() {
//         //     true => None,
//         //     false => Some(EmbeddingVector::from(embed(&self.content)?)),
//         // };
//         // let summary_embedding = match summary.is_empty() {
//         //     true => None,
//         //     false => Some(EmbeddingVector::from(embed(&summary)?)),
//         // };
//         let chunk = DocumentChunk {
//             range: (self.starting_line, self.ending_line),
//             content: self.content.to_owned(),
//             summary: None,
//             // summary_embedding,
//             // content_embedding,
//             changes: HashMap::new(),
//         };
//         Ok(chunk)
//     }
// }
//
impl DocumentChunk {
    fn new(starting_line: usize, ending_line: usize, content: String) -> Self {
        // DocumentChunkBuilder {
        //     starting_line,
        //     ending_line,
        //     content,
        // }
        Self {
            range: (starting_line, ending_line),
            changes: HashMap::new(),
            content,
            summary: None,
        }
    }

    /// USIZE TUPLE END INDEX _IS_ INCLUSIVE
    fn chunk_text(text: &str) -> Vec<((usize, usize), String)> {
        let mut chunks = vec![];
        let mut start = 0;
        let mut end = LINES_IN_CHUNK + start;
        let lines: Vec<&str> = text.lines().collect();
        while let Some(window) = {
            match (lines.get(start as usize), lines.get(end)) {
                (Some(_), Some(_)) => Some(lines[start..=end].to_owned()),
                (Some(_), None) => Some(lines[start..].to_owned()),
                _ => None,
            }
        } {
            chunks.push(((start, start + window.len() - 1), window.join("\n")));
            start += LINES_IN_CHUNK + chunks.len();
            end += start;
        }
        chunks
    }

    pub fn chunks_from_text(text: &str) -> Vec<DocumentChunk> {
        let mut chunks = vec![];
        for (range, text) in Self::chunk_text(text) {
            let chunk = DocumentChunk::new(range.0, range.1, text);
            chunks.push(chunk);
        }
        chunks
    }
}

#[cfg(test)]
mod tests {
    use super::DocumentChunk;
    #[tokio::test]
    async fn chunks_from_text_works() {
        let text = r#"
impl<'c> DocumentChunkBuilder<'c> {
    pub async fn build(self) -> Result<DocumentChunk, anyhow::Error> {
        let summary = summarize(Some(SUMMARIZE_DOC_CHUNK_PROMPT), &self.content)
            .await
            .unwrap();
        let content_embedding = EmbeddingVector::from(embed(&self.content)?);
        let summary_embedding = EmbeddingVector::from(embed(&summary)?);
        let chunk = DocumentChunk {
            range: (self.starting_line, self.ending_line),
            content: self.content.to_owned(),
            summary,
            summary_embedding,
            content_embedding,
            changes: HashMap::new(),
        };
        Ok(chunk)
    }
}

impl DocumentChunk {
    fn new<'c>(
        starting_line: usize,
        ending_line: usize,
        content: &str,
    ) -> DocumentChunkBuilder<'c> {
        DocumentChunkBuilder {
            starting_line,
            ending_line,
            content,
        }
    }

    pub async fn chunks_from_text(text: &str) -> Result<Vec<DocumentChunk>, anyhow::Error> {
        let mut chunks = vec![];
        let mut current_chunk: Option<DocumentChunkBuilder<'_>> = None;
        for (i, line) in text.split('\n').enumerate() {
            current_chunk = match current_chunk {
                Some(c) => {
                    let content = &format!("{}\n{}", c.content, line);
                    let ending = c.ending_line + 1;
                    Some(DocumentChunk::new(chunks.len(), ending, content))
                }
                None => Some(DocumentChunk::new(chunks.len(), i, "")),
            };
            if i % LINES_IN_CHUNK == 0 {
                chunks.push(current_chunk.take().unwrap().build().await?);
            }
        }
        Ok(chunks)
    }
}
            "#;

        let chunks = DocumentChunk::chunk_text(text);
        for (range, chunk) in chunks.iter() {
            println!("RANGE: {:?}", range);
            println!("CHUNK: {}", chunk);
            assert_eq!(
                chunk.lines().collect::<Vec<&str>>().len(),
                range.1 - range.0 + 1 // ADD ONE BECAUSE INCLUSIVE INDEX
            );
        }
    }
}
