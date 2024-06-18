use std::vec;

pub fn all_lines_with_pattern(pat: &str, text: &str) -> Vec<u32> {
    text.lines().enumerate().fold(vec![], |mut acc, (i, l)| {
        if l.contains(pat) {
            acc.push(i as u32)
        }
        acc
    })
}
