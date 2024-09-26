use crate::interact::comment_str_map::{get_comment_string_info, CommentStrInfo};
use lsp_types::{Position, Range};
use std::fmt::{Debug, Display};
use tracing::warn;
use tracing_subscriber::Registry;

use super::{methods::Interact, registry::InteractRegistry, InteractError, InteractResult};

#[derive(Debug)]
/// Over two lifetimes, 'i is the lifetime of the input string,
/// 'l is the lifetime of the lexer
pub struct Lexer<'i> {
    input: &'i str,
    comment_str_info: CommentStrInfo<'i>,
    buffer: String,
    position: usize,      // current position in input (points to current char)
    read_position: usize, // current reading position in input (after current char)
    ch: Option<char>,     // NONE if at end of input
    current_line: usize,
    current_char: usize,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ParsedComment {
    interact: Option<u8>,
    pub content: String,
    pub range: Range,
}

impl ParsedComment {
    /// returns range and text of comment without interract
    /// returns none if there is no interact code
    pub fn text_for_interact(&self) -> Option<(Range, String)> {
        self.interact.and_then(|_| {
            // for now all interact codes have only 2 chars, no more no less.
            // This will likely change in the future
            let chars_amt = 2;

            let whitespace_amt = self
                .content
                .chars()
                .position(|c| !c.is_whitespace())
                .unwrap();

            let mut range = self.range.clone();
            range.start.character += (chars_amt + whitespace_amt) as u32;
            let ret_str = self
                .content
                .chars()
                .skip(chars_amt + whitespace_amt)
                .collect();

            Some((range, ret_str))
        })
    }

    pub fn try_get_interact(&self) -> InteractResult<u8> {
        self.interact.ok_or(InteractError::NoInteractInComment)
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Token {
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

pub fn position_in_range(range: &Range, pos: &Position) -> bool {
    range.start.line <= pos.line && range.start.character <= pos.character
        || range.end.line >= pos.line && range.end.character >= pos.character
}

impl<'i> Lexer<'i> {
    pub fn new(input: &'i String, ext: &'i str) -> Self {
        let comment_str_info = get_comment_string_info(ext).expect("no comment string");

        Self {
            input,
            buffer: String::new(),
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

    pub fn lex_input(&mut self, registry: &InteractRegistry) -> Vec<Token> {
        let mut output = vec![];
        let mut start_opt = Option::<Position>::None;
        let mut end_opt = Option::<Position>::None;
        // let mut buffer = String::new();

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
            match c {
                _ if c == singleline_comment_first_char
                    || Some(c) == multiline_comment_start_first_char =>
                {
                    let singleline_start = self
                        .at_beginning_of_slice(&singleline_comment_str)
                        .to_owned();

                    let multiline_start = multiline_comment_start_str
                        .as_ref()
                        .and_then(|slice| Some(self.at_beginning_of_slice(&slice)))
                        .unwrap_or(false);

                    if !singleline_start && !multiline_start {
                        self.buffer.push(c);
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

                    let end_slice_first_char = end_slice
                        .chars()
                        .nth(0)
                        .expect("end slice should not be an empty string");

                    if !self.buffer.is_empty() {
                        output.push(Token::Block(self.buffer.drain(..).collect::<String>()));
                    }

                    for _ in 0..start_slice.len() {
                        self.progress_char();
                    }

                    start_opt = Some(self.current_position());
                    output.push(Token::CommentStr);

                    while let Some(peek) = self.peek() {
                        self.buffer.push(self.ch.unwrap());
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

                    let content = self.buffer.drain(..).collect::<String>();

                    let interact = registry.try_get_interact(&content);

                    output.push(Token::Comment(ParsedComment {
                        interact,
                        content,
                        range,
                    }));

                    if multiline_start {
                        output.push(Token::CommentStr);
                    };
                }

                '\n' => {
                    self.buffer.push(c);
                    if self.peek() == Some('\n') {
                        while self.peek() == Some('\n') {
                            self.progress_char();
                            self.buffer.push(self.ch.expect("this should be some"));
                        }
                        output.push(Token::Block(self.buffer.drain(..).collect::<String>()))
                    }
                }

                _ => {
                    self.buffer.push(c);
                }
            }
            self.progress_char();
        }

        if !self.buffer.is_empty() {
            output.push(Token::Block(self.buffer.drain(..).collect()));
        }

        output.push(Token::End);

        output
    }
}

mod tests {

    use super::{Lexer, ParsedComment, Token};
    use crate::interact::{
        lexer::position_in_range,
        methods::{COMMAND_PROMPT, SCOPE_GLOBAL},
        registry::InteractRegistry,
    };
    use lsp_types::{Position, Range};
    use tracing::warn;

    #[test]
    fn pos_in_range_works() {
        let pos = Position {
            line: 2,
            character: 4,
        };

        let range = Range {
            start: Position {
                line: 1,
                character: 0,
            },
            end: Position {
                line: 11,
                character: 0,
            },
        };

        assert!(position_in_range(&range, &pos));

        let pos = Position {
            line: 1,
            character: 4,
        };

        let range = Range {
            start: Position {
                line: 3,
                character: 0,
            },
            end: Position {
                line: 11,
                character: 0,
            },
        };

        assert!(!position_in_range(&range, &pos));

        let pos = Position {
            line: 1,
            character: 4,
        };

        let range = Range {
            start: Position {
                line: 1,
                character: 4,
            },
            end: Position {
                line: 11,
                character: 0,
            },
        };

        assert!(position_in_range(&range, &pos));

        let pos = Position {
            line: 11,
            character: 4,
        };

        let range = Range {
            start: Position {
                line: 1,
                character: 4,
            },
            end: Position {
                line: 11,
                character: 4,
            },
        };

        assert!(position_in_range(&range, &pos));
    }

    #[test]
    fn at_begginning_of_slice_works() {
        let input = "pub mod lexer;".to_owned();
        let mut lexer = Lexer::new(&input, "rs");

        lexer.progress_char();
        lexer.progress_char();
        lexer.progress_char();
        lexer.progress_char();

        assert!(lexer.at_beginning_of_slice("mod"));

        let input = "\n".to_owned();
        let lexer = Lexer::new(&input, "rs");
        assert!(lexer.at_beginning_of_slice("\n"));
    }

    #[test]
    fn lexing_rust_comments_works() {
        let input = r#"
pub mod lexer;
use std::sync::LazyLock;

use lsp_types::Range;

use super::{InteractError, InteractResult};

// @_Comment
pub struct ParsedComment {
    content: String,
    range: Range,
}

/*
Multiline
comment
*/
pub struct MoreCode;
        "#
        .to_owned();

        let mut lexer = Lexer::new(&input, "rs");
        warn!("created lexer: {lexer:?}");
        let registry = InteractRegistry::default();
        let tokens = lexer.lex_input(&registry);
        let expected = vec![
            Token::Block(String::from(
                "\npub mod lexer;\nuse std::sync::LazyLock;\n\n",
            )),
            Token::Block(String::from("use lsp_types::Range;\n\n")),
            Token::Block(String::from(
                "use super::{InteractError, InteractResult};\n\n",
            )),
            Token::CommentStr,
            Token::Comment(ParsedComment {
                interact: Some(COMMAND_PROMPT + SCOPE_GLOBAL),
                content: " @_Comment".to_string(),
                range: lsp_types::Range {
                    start: lsp_types::Position {
                        line: 8,
                        character: 3,
                    },
                    end: lsp_types::Position {
                        line: 8,
                        character: 13,
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
                interact: None,
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

        let all = tokens.into_iter().zip(expected);

        for (token, exp) in all {
            assert_eq!(token, exp)
        }
    }
}
