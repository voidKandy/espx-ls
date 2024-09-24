use crate::commands::comment_str_map::{get_comment_string_info, CommentStrInfo};
use lsp_types::{Position, Range};
use std::fmt::{Debug, Display};

#[derive(Debug)]
struct Lexer {
    input: String,
    position: usize,      // current position in input (points to current char)
    read_position: usize, // current reading position in input (after current char)
    ch: Option<char>,     // NONE if at end of input
    comment_str_info: CommentStrInfo,
    current_line: usize,
    current_char: usize,
}

#[derive(Debug, PartialEq, Eq)]
pub(super) struct ParsedComment {
    content: String,
    range: Range,
}

#[derive(Debug, PartialEq, Eq)]
enum Token {
    CommentStr,
    Comment(ParsedComment),
    Block(String),
    End,
}

impl Display for Token {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let str = match self {
            Self::End => "End",
            Self::CommentStr => "CommentStr",
            Self::Block(_) => "Block(String)",
            Self::Comment(_) => "Comment(ParsedComment)",
        };
        write!(f, "{str}")
    }
}

impl Lexer {
    pub fn new(input: &str, ext: &str) -> Self {
        let comment_str_info = get_comment_string_info(ext).expect("no comment string");

        Self {
            input: input.to_owned(),
            position: 0,
            read_position: 1,
            ch: input.to_lowercase().chars().nth(0),
            comment_str_info,
            current_line: 0,
            current_char: 0,
        }
    }

    /// Checks that the current char is indeed the beginning of a slice that will result in the
    /// given &str
    fn at_beginning_of_slice(&self, slice: &str) -> bool {
        let mut chars = slice.chars();
        let mut i = 0;
        let first = chars
            .next()
            .expect("should not pass an empty string to this function");

        if self.ch != Some(first) {
            return false;
        }

        if slice.len() == 1 {
            return true;
        }

        while let Some(next_expected_char) = chars.next() {
            if let Some(next_peek) = self.peek_offset(i) {
                if next_peek != next_expected_char {
                    return false;
                }
            }
            i += 1;
        }
        if let Some(n) = self.peek_offset(i) {
            return n.is_whitespace();
        }
        true
    }

    // fn comment_str_char(&self, multiline_start: Option<bool>, char_idx: usize) -> Option<char> {
    //     match multiline_start {
    //         Some(is_start) => {
    //             if is_start {
    //                 self.comment_str_info
    //                     .multiline_start()
    //                     .and_then(|str| str.chars().nth(char_idx))
    //             } else {
    //                 self.comment_str_info
    //                     .multiline_end()
    //                     .and_then(|str| str.chars().nth(char_idx))
    //             }
    //         }
    //         None => self.comment_str_info.singleline().chars().nth(char_idx),
    //     }
    // }

    fn comment_str(&self, multiline_start: Option<bool>) -> Option<&str> {
        match multiline_start {
            None => Some(self.comment_str_info.singleline()),
            Some(is_start) => {
                if is_start {
                    self.comment_str_info.multiline_start()
                } else {
                    self.comment_str_info.multiline_end()
                }
            }
        }
    }

    fn current_position(&self) -> Position {
        Position {
            line: self.current_line as u32,
            character: self.current_char as u32,
        }
    }

    fn progress_char(&mut self) {
        if self.ch == Some('\n') {
            self.current_line += 1;
            self.current_char = 0;
        }
        self.ch = self.input.chars().nth(self.read_position);
        self.position = self.read_position;
        self.current_char += 1;
        self.read_position += 1
    }

    fn peek(&self) -> Option<char> {
        self.input.chars().nth(self.read_position)
    }

    fn peek_offset(&self, offset: usize) -> Option<char> {
        self.input.chars().nth(self.read_position + offset)
    }

    pub fn lex_input(&mut self) -> anyhow::Result<Vec<Token>> {
        let mut output = vec![];
        let mut start_opt = Option::<Position>::None;
        let mut end_opt = Option::<Position>::None;
        let mut buffer = String::new();

        let singleline_comment_str = self.comment_str(None).unwrap().to_owned();
        let singleline_comment_first_char =
            singleline_comment_str.chars().nth(0).unwrap().to_owned();

        let multiline_comment_start_str = self
            .comment_str(Some(true))
            .and_then(|s| Some(s.to_owned()));
        let multiline_comment_start_first_char = multiline_comment_start_str
            .as_ref()
            .and_then(|str| str.chars().nth(0).and_then(|i| Some(i.to_owned())));

        while let Some(c) = self.ch {
            println!("char: {c}");
            match c {
                _ if c == singleline_comment_first_char
                    || Some(c) == multiline_comment_start_first_char =>
                {
                    println!("in the comment start branch");
                    let singleline_start = self
                        .at_beginning_of_slice(&singleline_comment_str)
                        .to_owned();

                    println!("matches singleline: {singleline_start}");

                    let multiline_start = multiline_comment_start_str
                        .as_ref()
                        .and_then(|slice| Some(self.at_beginning_of_slice(&slice)))
                        .unwrap_or(false);
                    println!("matches multiline: {multiline_start}");

                    if !singleline_start && !multiline_start {
                        buffer.push(c);
                        self.progress_char();
                        continue;
                    }

                    let (start_slice, end_slice) = {
                        if singleline_start {
                            (&singleline_comment_str, "\n".to_owned())
                        } else {
                            (
                                multiline_comment_start_str
                                    .as_ref()
                                    .expect("should be some in this branch"),
                                self.comment_str(Some(false))
                                    .expect("this should be some in this branch")
                                    .to_owned(),
                            )
                        }
                    };

                    println!("start slice: {start_slice}\nend slice: {end_slice}");

                    let end_slice_first_char = end_slice
                        .chars()
                        .nth(0)
                        .expect("end slice should not be an empty string");

                    if !buffer.is_empty() {
                        output.push(Token::Block(buffer.drain(..).collect()));
                    }

                    for _ in 0..start_slice.len() {
                        self.progress_char();
                    }

                    start_opt = Some(self.current_position());
                    output.push(Token::CommentStr);

                    println!("began iterating through until end slice: {end_slice:?}");
                    while let Some(peek) = self.peek() {
                        println!("current peek: {peek:?}");
                        buffer.push(self.ch.unwrap());
                        self.progress_char();
                        if peek == end_slice_first_char {
                            if self.at_beginning_of_slice(&end_slice) {
                                end_opt = Some(self.current_position());
                                for _ in 0..end_slice.len() - 1 {
                                    self.progress_char();
                                }
                                break;
                            }
                        }
                    }

                    let range = Range {
                        start: start_opt.expect("start opt should be some at this point"),
                        end: end_opt.expect("end opt should be some at this point"),
                    };

                    println!("range for comment: {range:?}");

                    output.push(Token::Comment(ParsedComment {
                        content: buffer.drain(..).collect(),
                        range,
                    }));

                    if multiline_start {
                        output.push(Token::CommentStr);
                    };
                }

                '\n' => {
                    buffer.push(c);
                    if self.peek() == Some('\n') {
                        while self.peek() == Some('\n') {
                            self.progress_char();
                            buffer.push(self.ch.expect("this should be some"));
                        }
                        output.push(Token::Block(buffer.drain(..).collect()))
                    }
                }

                _ => {
                    buffer.push(c);
                }
            }
            self.progress_char();
        }

        if !buffer.is_empty() {
            output.push(Token::Block(buffer));
        }

        output.push(Token::End);

        println!("returning {output:?}");
        Ok(output)
    }
}

mod tests {

    use crate::tests::init_test_tracing;

    use super::{Lexer, ParsedComment, Token};

    #[test]
    fn at_begginning_of_slice_works() {
        let input = "pub mod lexer;";
        let mut lexer = Lexer::new(input, "rs");

        lexer.progress_char();
        lexer.progress_char();
        lexer.progress_char();
        lexer.progress_char();

        assert!(lexer.at_beginning_of_slice("mod"));

        let input = "\n";
        let mut lexer = Lexer::new(input, "rs");
        assert!(lexer.at_beginning_of_slice("\n"));
    }

    #[test]
    fn lexing_rust_comments_works() {
        init_test_tracing();
        let input = r#"
pub mod lexer;
use std::sync::LazyLock;

use lsp_types::Range;

use super::{CommandError, CommandResult};

// Comment
pub struct ParsedComment {
    content: String,
    range: Range,
}

/*
Multiline
comment
*/
pub struct MoreCode;
        "#;

        let mut lexer = Lexer::new(input, "rs");
        println!("created lexer: {lexer:?}");
        let tokens = lexer.lex_input().unwrap();
        let expected = vec![
            Token::Block(String::from(
                "\npub mod lexer;\nuse std::sync::LazyLock;\n\n",
            )),
            Token::Block(String::from("use lsp_types::Range;\n\n")),
            Token::Block(String::from(
                "use super::{CommandError, CommandResult};\n\n",
            )),
            Token::CommentStr,
            Token::Comment(ParsedComment {
                content: " Comment".to_string(),
                range: lsp_types::Range {
                    start: lsp_types::Position {
                        line: 8,
                        character: 3,
                    },
                    end: lsp_types::Position {
                        line: 8,
                        character: 11,
                    },
                },
            }),
            Token::Block(String::from(
                r#"pub struct ParsedComment {
    content: String,
    range: Range,
}

"#,
            )),
            Token::CommentStr,
            Token::Comment(ParsedComment {
                content: "\nMultiline\ncomment\n".to_string(),
                range: lsp_types::Range {
                    start: lsp_types::Position {
                        line: 14,
                        character: 3,
                    },
                    end: lsp_types::Position {
                        line: 17,
                        character: 1,
                    },
                },
            }),
            Token::CommentStr,
            Token::Block(String::from(
                r#"
pub struct MoreCode;
        "#,
            )),
            Token::End,
        ];

        // if tokens != expected {
        //     println!("\ntokens:");
        //     for t in tokens {
        //         println!("{t}");
        //     }
        //
        //     println!("\nexpected:");
        //     for t in expected {
        //         println!("{t}");
        //     }
        //
        //     panic!("tokens != expected")
        // }
        assert_eq!(tokens, expected);
    }
}
