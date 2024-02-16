use log::debug;
use lsp_server::{Request, RequestId};
use lsp_types::{
    CodeLensParams, Position as LspPos, TextDocumentPositionParams, WorkDoneProgressParams,
};
use nom::{
    self,
    bytes::complete::{tag, take_till, take_until},
    character::{complete::newline, is_newline},
    sequence::{delimited, preceded},
    IResult,
};
use uuid::Uuid;

use crate::text_store::get_text_document_current;

#[derive(Debug, Clone, PartialEq)]
pub enum Position {
    UserPrompt(String),
    // AttributeValue { name: String, value: String },
}

const PREFIX: &str = "#$";
pub fn parse_for_prompt(input: &str) -> IResult<&str, &str> {
    let (r, o) = preceded(
        take_until(PREFIX),
        take_till(|c| is_newline(c as u8)), // tag("p"),
    )(input)?;
    let o = o.strip_prefix(PREFIX).unwrap();
    Ok((r, o))
}

pub fn get_prompt_and_position(input: &str) -> Option<(String, LspPos)> {
    for (i, l) in input.lines().into_iter().enumerate() {
        if let Some(idx) = l.find(PREFIX) {
            let (_, o) = parse_for_prompt(l).unwrap();
            let pos = LspPos {
                line: i as u32,
                character: (idx + o.len()) as u32,
            };
            return Some((o.to_string(), pos));
        }
    }
    None
}

// pub fn parse_for_prompt_prefix<'a>(i: &'a str) -> IResult<&'a str, &'a str> {
//     terminated(tag("π"), is_not("\n\r")).parse(i)
// }

pub fn get_position_from_lsp_completion(
    text_params: &TextDocumentPositionParams,
) -> Option<Position> {
    debug!(
        "get_position_from_lsp_completion: uri {}",
        text_params.text_document.uri
    );
    let text = get_text_document_current(&text_params.text_document.uri)?;
    debug!("get_position_from_lsp_completion: text {}", text);
    let pos = text_params.position;
    debug!("get_position_from_lsp_completion: pos {:?}", pos);

    match parse_for_prompt(&text) {
        Ok((_, out)) => {
            // debug!("Parsed text output!: {:?}", t);
            Some(Position::UserPrompt(out.to_string()))
        }
        Err(err) => {
            debug!("Error parsing text: {:?}", err);
            None
        }
    }

    // if text.contains(prompt_pattern) {
    //     Some(Position::UserPrompt("hx-special-test".to_string()))
    // } else {
    //     None
    // }
}

#[cfg(test)]
mod tests {
    use crate::parsing::get_prompt_and_position;

    use super::parse_for_prompt;

    #[test]
    fn parse_for_prompt_gets_correct_prompt() {
        let input = "not a prompt #$ This is a prompt 
            notAprompt";
        // let input = "#$phij";
        let (i, o) = parse_for_prompt(&input).unwrap();
        println!("I: {},O: {}", i, o);
        assert_eq!(" This is a prompt", o);
    }

    #[test]
    fn output_pos_is_correct() {
        let input = "
not a prompt
Not a prompt
#$ This is a prompt 
notAprompt";
        let (_, pos) = get_prompt_and_position(input).unwrap();
        println!("{:?}", pos);
        assert_eq!((3, 18), (pos.line, pos.character));
    }
}
