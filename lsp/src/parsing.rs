use crate::{ handle::runes::BasicRune};
use lsp_types::Url;

use nom::{
    self,
    bytes::complete::{take_till, take_until},
    character::is_newline,
    sequence::preceded,
    IResult,
};

pub fn get_prompt_on_line(input: &str, prefix: &str) -> Option<(String, String)> {
    if let Ok((_, o)) = parse_for_prompt(input, prefix) {
        let pre_prompt_text = input.split_once(prefix).unwrap().0.to_owned();
        return Some((pre_prompt_text, o.to_owned()));
    }
    None
}

fn parse_for_prompt<'i>(input: &'i str, prefix: &str) -> IResult<&'i str, &'i str> {
    let (r, o) = preceded(
        take_until(prefix),
        take_till(|c| is_newline(c as u8)), // tag("p"),
    )(input)?;
    let o = o.strip_prefix(prefix).unwrap();
    Ok((r, o))
}

fn get_runes_in_doc<'i>(
    input: &'i str,
    rune: impl BasicRune,
    url: &'i Url,
) -> Option<&'i Box<dyn BasicRune>> {
    let runes = GLOBAL_CACHE.read().unwrap().runes;
    if let Ok(_) = parse_for_rune(input, rune_placeholder) {
        match runes.get(url) {
Some(doc_rune_map) => {
            return doc_rune_map.get(rune_placeholder);

            }
            None => {}
        }
        if let  = runes.get(url) {
        }
    }
    None
}

fn parse_for_rune<'i>(input: &'i str, rune_placeholder: &str) -> IResult<&'i str, &'i str> {
    let (r, o) = preceded(
        take_until(rune_placeholder),
        take_till(|c| is_newline(c as u8)), // tag("p"),
    )(input)?;
    let o = o.strip_prefix(rune_placeholder).unwrap();

    Ok((r, o))
}
