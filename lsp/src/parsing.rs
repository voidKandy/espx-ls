use anyhow::anyhow;
use lsp_types::Position as LspPos;
use nom::{
    self,
    bytes::complete::{take_till, take_until},
    character::is_newline,
    sequence::preceded,
    IResult,
};

use crate::config::UserActionConfig;

pub struct UserActionExecutionParams {
    pub replacement_text: String,
    pub prompt: String,
    pub pos: LspPos,
}

pub enum UserAction {
    IoPrompt(UserActionExecutionParams),
}

impl<'ac> Into<Vec<&'ac str>> for &'ac UserActionConfig {
    fn into(self) -> Vec<&'ac str> {
        vec![self.io_trigger.as_str()]
    }
}

impl UserActionExecutionParams {
    pub fn get_prompt_on_line(input: &str, prefix: &str) -> Option<(String, String)> {
        if let Ok((_, o)) = Self::parse_for_prompt(input, prefix) {
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
}

impl TryFrom<(&str, &str)> for UserActionExecutionParams {
    type Error = anyhow::Error;
    fn try_from((input, prefix): (&str, &str)) -> Result<Self, Self::Error> {
        for (i, l) in input.lines().into_iter().enumerate() {
            if l.contains(&prefix) {
                if let Some((replacement_text, prompt)) =
                    UserActionExecutionParams::get_prompt_on_line(l, prefix)
                {
                    let pos = LspPos {
                        line: i as u32,
                        character: prompt.len() as u32,
                    };
                    return Ok(UserActionExecutionParams {
                        replacement_text,
                        prompt,
                        pos,
                    });
                }
            }
        }
        Err(anyhow!("None Found"))
    }
}

impl<'ac> UserAction {
    pub fn all_actions_in_text(config: &'ac UserActionConfig, input: &'ac str) -> Vec<UserAction> {
        let mut action_vec = vec![];
        let mut trigger_list: Vec<&str> = config.into();
        while let Some(trig) = trigger_list.pop() {
            if let Some(params) = UserActionExecutionParams::try_from((input, trig)).ok() {
                match trig {
                    _io if trig == config.io_trigger => {
                        action_vec.push(UserAction::IoPrompt(params));
                    }
                    _ => {}
                }
            }
        }

        action_vec
    }
}

#[cfg(test)]
mod tests {
    use crate::{
        config::UserActionConfig,
        parsing::{UserAction, UserActionExecutionParams},
    };

    #[test]
    fn parse_for_prompt_gets_correct_prompt() {
        let input = "not a prompt #$ This is a prompt
            notAprompt";
        let (i, o) = UserActionExecutionParams::parse_for_prompt(&input, "#$").unwrap();
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
        let config = UserActionConfig::default();
        let all_actions = UserAction::all_actions_in_text(&config, input);
        assert_eq!(all_actions.len(), 1);
    }
}
