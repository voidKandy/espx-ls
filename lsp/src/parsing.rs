use log::{debug, error};
use lsp_types::TextDocumentPositionParams;
use nom::{
    self,
    bytes::complete::{tag, take, take_until},
    character::complete::{alpha0, anychar, line_ending},
    sequence::delimited,
    IResult,
};

use crate::text_store::get_text_document;

#[derive(Debug, Clone, PartialEq)]
pub enum Position {
    AttributeName(String),
    AttributeValue { name: String, value: String },
}

// Should have a const list of commentstrings that will be prepended to the tag based on filetype??
const RUST_COMMENTSTRING: &'static str = "//";

fn parse_for_prompt_prefix(input: &str) -> IResult<&str, &str> {
    let (r, o) = delimited(tag("//"), take_until("//"), tag("//"))(input)?;
    Ok((r, o.trim()))
}

pub fn get_position_from_lsp_completion(
    text_params: TextDocumentPositionParams,
) -> Option<Position> {
    debug!(
        "get_position_from_lsp_completion: uri {}",
        text_params.text_document.uri
    );
    let text = get_text_document(text_params.text_document.uri)?;
    debug!("get_position_from_lsp_completion: text {}", text);
    let pos = text_params.position;
    debug!("get_position_from_lsp_completion: pos {:?}", pos);

    match parse_for_prompt_prefix(&text) {
        Ok((_, output)) => {
            debug!("Parsed text output!: {:?}", output);
            Some(Position::AttributeName(output.to_string()))
        }
        Err(err) => {
            debug!("Error parsing text: {:?}", err);
            None
        }
    }

    // if text.contains(prompt_pattern) {
    //     Some(Position::AttributeName("hx-special-test".to_string()))
    // } else {
    //     None
    // }
}

#[cfg(test)]
mod tests {
    use super::parse_for_prompt_prefix;

    #[test]
    fn prompt_prefix_gets_correct_prompt() {
        let input = "// This is a prompt //";
        let (_, output) = parse_for_prompt_prefix(&input).unwrap();
        assert_eq!("This is a prompt", output);
    }
}
