mod tree_sitter;
use nom::{
    self,
    bytes::complete::{take_till, take_until},
    character::is_newline,
    sequence::preceded,
    IResult,
};

// pub fn parse_for_position

pub fn get_prompt_on_line(input: &str, prefix: &str) -> Option<(String, String)> {
    if let Ok((_, o)) = parse_for_prompt(input, prefix) {
        let pre_prompt_text = input.split_once(prefix).unwrap().0.to_owned();
        let prompt = o.trim();
        // let prompt = o.strip_prefix("\"").unwrap_or(prompt);
        // let prompt = o.strip_suffix("\"").unwrap_or(prompt);
        return Some((pre_prompt_text, prompt.to_owned()));
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

// fn get_runes_in_doc<'i>(
//     input: &'i str,
//     rune_placeholder: char,
//     url: &'i Url,
// ) -> Option<&'i BufferBurn> {
//     let runes = GLOBAL_CACHE.read().unwrap().runes;
//     if let Ok(_) = parse_for_rune(input, rune_placeholder) {
//         if let Some(doc_rune_map) = runes.get(url) {
//             return doc_rune_map.get(&rune_placeholder);
//         }
//     }
//     None
// }

fn parse_for_rune<'i>(input: &'i str, rune_placeholder: char) -> IResult<&'i str, &'i str> {
    let (r, o) = preceded(
        take_until(String::from(rune_placeholder).as_str()),
        take_till(|c| is_newline(c as u8)), // tag("p"),
    )(input)?;
    let o = o.strip_prefix(rune_placeholder).unwrap();

    Ok((r, o))
}
