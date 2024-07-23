use std::vec;

use anyhow::anyhow;
use futures::future::join;
use lsp_types::{Position, Range};
use tracing::{debug, warn};
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct TextOnLineRange {
    pub range: Range,
    pub text: String,
}

/// Returns a vec of tuples containing lines
pub fn all_lines_with_pattern(text: &str, pat: &str) -> Vec<usize> {
    text.lines().enumerate().fold(vec![], |mut acc, (i, l)| {
        if l.contains(pat) {
            acc.push(i as usize)
        }
        acc
    })
}

/// Returns a vec of tuples containing lines and position of first char
pub fn all_lines_with_pattern_with_char_positions(text: &str, pat: &str) -> Vec<(usize, usize)> {
    let mut all = vec![];
    for (i, l) in text.lines().enumerate() {
        let mut line = l.to_string();
        let mut prev_skip = 0;
        while let Some(mut idx) = line.find(pat) {
            idx = prev_skip + idx;
            all.push((i, idx));
            line = line.chars().skip(idx + 1).collect::<String>();
            prev_skip = idx + 1;
        }
    }
    all
}

pub fn slices_of_pattern(text: &str, pat: &str) -> Option<Vec<TextOnLineRange>> {
    let lines_and_start_posis = all_lines_with_pattern_with_char_positions(text, pat);
    if lines_and_start_posis.is_empty() {
        debug!("No line matching pattern");
        return None;
    }

    let slices = lines_and_start_posis.into_iter().fold(
        Vec::<TextOnLineRange>::new(),
        |mut slices, (l, char)| {
            slices.push(TextOnLineRange {
                range: Range {
                    start: Position {
                        line: l as u32,
                        character: char as u32,
                    },
                    end: Position {
                        line: l as u32,
                        character: char as u32 + pat.len() as u32,
                    },
                },
                text: pat.to_string(),
            });
            slices
        },
    );
    if slices.is_empty() {
        return None;
    }
    Some(slices)
}

pub fn slices_after_pattern(text: &str, pat: &str) -> Option<Vec<TextOnLineRange>> {
    let lines_and_start_posis = all_lines_with_pattern_with_char_positions(text, pat);
    if lines_and_start_posis.is_empty() {}

    let lines: Vec<&str> = text.lines().collect();
    let slices = lines_and_start_posis.into_iter().fold(
        Vec::<TextOnLineRange>::new(),
        |mut slices, (l, char)| {
            let line = lines[l as usize];
            let end_of_pat_pos = char + pat.len();
            let content = &line[end_of_pat_pos + 1..];
            let end_of_slice_pos = end_of_pat_pos + content.len();
            slices.push(TextOnLineRange {
                range: Range {
                    start: Position {
                        line: l as u32,
                        character: end_of_pat_pos as u32 + 1,
                    },
                    end: Position {
                        line: l as u32,
                        character: end_of_slice_pos as u32 + 1,
                    },
                },
                text: content.to_string(),
            });
            slices
        },
    );

    if slices.is_empty() {
        return None;
    }
    Some(slices)
}

pub fn slices_between_pattern(text: &str, pat: &str) -> Option<Vec<TextOnLineRange>> {
    let mut lines_and_start_posis = all_lines_with_pattern_with_char_positions(text, pat);
    if lines_and_start_posis.is_empty() {
        return None;
    }

    if lines_and_start_posis.len() % 2 != 0 {
        warn!("Uneven amount of patterns! Slices may return unexpectedly")
    }

    let mut slices = vec![];

    for _ in 0..lines_and_start_posis.len() / 2 {
        let (first_line_idx, first_char_idx) = lines_and_start_posis.remove(0);
        let (last_line_idx, last_char_idx) = lines_and_start_posis.remove(0);

        // let buffer = {
        if first_line_idx == last_line_idx {
            if let Some(buffer) = text.lines().nth(first_line_idx).map(|l| {
                l.chars()
                    .skip(first_char_idx + pat.len() + 1)
                    .take(last_char_idx - pat.len() - 2)
                    .collect::<String>()
            }) {
                slices.push(TextOnLineRange {
                    range: Range {
                        start: Position {
                            line: first_line_idx as u32,
                            character: (first_char_idx + pat.len()) as u32 + 1,
                        },
                        end: Position {
                            line: (first_line_idx + buffer.lines().count()) as u32 - 1,
                            character: (buffer.len() + first_char_idx + pat.len()) as u32 + 1,
                        },
                    },
                    text: buffer,
                })
            }
        } else {
            let b = text
                .lines()
                .skip(first_line_idx as usize)
                .take(last_line_idx - first_line_idx)
                .collect::<Vec<&str>>()
                .join("\n");
            debug!("b: {:?}", b);
            debug!(
                "firstchar: {} firstline: {} lastchar: {} lastline: {}",
                first_char_idx, first_line_idx, last_char_idx, last_line_idx
            );

            let buffer = b
                .trim()
                .chars()
                .skip(first_char_idx + pat.len())
                .take(last_char_idx + pat.len() + b.len() + last_line_idx)
                .collect::<String>();
            if !buffer.is_empty() {
                debug!("pushing multiline slice");
                slices.push(TextOnLineRange {
                    range: Range {
                        start: Position {
                            line: first_line_idx as u32,
                            character: (first_char_idx + pat.len()) as u32 + 1,
                        },
                        end: Position {
                            line: (first_line_idx + buffer.lines().count()) as u32 - 1,
                            character: buffer.lines().last().unwrap_or("").len() as u32,
                        },
                    },
                    text: buffer,
                })
            }
        }
        // };
    }
    if slices.is_empty() {
        debug!("slices are empty, returning None");
        return None;
    }
    Some(slices)
}
