use std::vec;

pub fn all_lines_with_pattern(pat: &str, text: &str) -> Vec<u32> {
    text.lines().enumerate().fold(vec![], |mut acc, (i, l)| {
        if l.contains(pat) {
            acc.push(i as u32)
        }
        acc
    })
}

/// returns an array of tuples: (line, char)
pub fn all_lines_with_pattern_with_char_positions(pat: &str, text: &str) -> Vec<(u32, u32)> {
    text.lines().enumerate().fold(vec![], |mut acc, (i, l)| {
        if let Some(idx) = l.find(pat) {
            acc.push((i as u32, idx as u32))
        }
        acc
    })
}

mod tests {
    use crate::parsing::all_lines_with_pattern_with_char_positions;

    #[test]
    fn lines_with_chars_works() {
        let input = r#"
text, more text

And some more text

and =>= pattern

and another instance: 

=>=
        "#;
        let expected = vec![(5, 4), (9, 0)];
        let out = all_lines_with_pattern_with_char_positions("=>=", &input);
        assert_eq!(out, expected)
    }
}
