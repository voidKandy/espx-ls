use crate::parsing::{
    all_lines_with_pattern, all_lines_with_pattern_with_char_positions, slices_after_pattern,
    slices_between_pattern, slices_of_pattern, TextOnLineRange,
};
use lsp_types::{Position, Range};

#[test]
fn lines_works() {
    let input = r#"
text, more text
=>=
And some more text

and =>= pattern

and another instance: 

=>=
        "#;
    let expected = vec![2, 5, 9];
    let out = all_lines_with_pattern(&input, "=>=");
    assert_eq!(out, expected)
}

#[test]
fn lines_with_chars_works() {
    let input = r#"
text, more text

And some more text

and =>= pattern

and another instance: 

=>= some mor =>=
        "#;
    let expected = vec![(5, 4), (9, 0), (9, 13)];
    let out = all_lines_with_pattern_with_char_positions(&input, "=>=");
    assert_eq!(out, expected)
}

#[test]
fn slices_of_pattern_works() {
    let input = r#"
text, more text
#$ pattern was here

text
test
textgas
#$ pattern also here
        "#;
    let expected = vec![
        TextOnLineRange {
            range: Range {
                start: Position {
                    line: 2,
                    character: 0,
                },
                end: Position {
                    line: 2,
                    character: 2,
                },
            },
            text: "#$".to_string(),
        },
        TextOnLineRange {
            range: Range {
                start: Position {
                    line: 7,
                    character: 0,
                },
                end: Position {
                    line: 7,
                    character: 2,
                },
            },
            text: "#$".to_string(),
        },
    ];
    let out = slices_of_pattern(&input, "#$").unwrap();
    for i in 0..expected.len() {
        assert_eq!(out[i], expected[i]);
    }
}

#[test]
fn slices_after_pattern_works() {
    let input = r#"
text, more text
#$ pattern was here

text
test
textgas
⚑ pattern also here
        "#;
    let expected = vec![
        TextOnLineRange {
            range: Range {
                start: Position {
                    line: 2,
                    character: 3,
                },
                end: Position {
                    line: 2,
                    character: 19,
                },
            },
            text: "pattern was here".to_string(),
        },
        TextOnLineRange {
            range: Range {
                start: Position {
                    line: 7,
                    character: 2,
                },
                end: Position {
                    line: 7,
                    character: 19,
                },
            },
            text: "pattern also here".to_string(),
        },
    ];
    let out = slices_after_pattern(&input, "#$").unwrap();
    assert_eq!(out[0], expected[0]);
    let out = slices_after_pattern(&input, "⚑").unwrap();
    assert_eq!(out[0], expected[1]);
}

#[test]
fn slices_between_pattern_works() {
    // super::init_test_tracing();
    let input = r#"
text, more text
#$ pattern was here #$

text
test
textgas
#$ pattern also here #$

#$
this is a multi
line 
pattern 

;)
#$
        "#;
    let expected = vec![
        TextOnLineRange {
            range: Range {
                start: Position {
                    line: 2,
                    character: 3,
                },
                end: Position {
                    line: 2,
                    character: 19,
                },
            },
            text: "pattern was here".to_string(),
        },
        TextOnLineRange {
            range: Range {
                start: Position {
                    line: 7,
                    character: 3,
                },
                end: Position {
                    line: 7,
                    character: 20,
                },
            },
            text: "pattern also here".to_string(),
        },
        TextOnLineRange {
            range: Range {
                start: Position {
                    line: 9,
                    character: 3,
                },
                end: Position {
                    line: 14,
                    character: 2,
                },
            },
            text: r#"
this is a multi
line 
pattern 

;)"#
            .to_string(),
        },
    ];
    let out = slices_between_pattern(&input, "#$").unwrap();
    for i in 0..expected.len() {
        assert_eq!(out[i], expected[i]);
    }
}
