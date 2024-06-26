use std::str::FromStr;

use crate::{
    config::{self, GLOBAL_CONFIG},
    handle::buffer_operations::{BufferOpChannelSender, BufferOperation},
};
use anyhow::anyhow;
use espionox::{language_models::completions::streaming::ProviderStreamHandler, prelude::*};
use lsp_server::RequestId;
use lsp_types::{
    GotoDefinitionResponse, HoverContents, Location, Range, Uri, WorkDoneProgress,
    WorkDoneProgressBegin,
};
use serde::{Deserialize, Serialize};
use tracing::debug;

#[derive(Clone, Serialize, Deserialize, Debug)]
pub enum SingleLineBurn {
    QuickPrompt {
        hover_contents: Option<HoverContents>,
    },
    RagPrompt {
        hover_contents: Option<HoverContents>,
    },
    WalkProject {
        hover_contents: Option<HoverContents>,
    },
}

#[derive(Debug, PartialEq, Eq)]
pub struct TextAndCharRange {
    pub text: String,
    pub start: u32,
    pub end: u32,
}

impl BurnActivation {
    pub fn all_variants_empty() -> Vec<Self> {
        vec![
            Self::RagPrompt {
                hover_contents: None,
            },
            Self::QuickPrompt {
                hover_contents: None,
            },
            Self::WalkProject {
                hover_contents: None,
            },
        ]
    }
    pub fn save_hover_contents(&mut self, contents: HoverContents) {
        match self {
            Self::RagPrompt { hover_contents } => *hover_contents = Some(contents),
            Self::QuickPrompt { hover_contents } => *hover_contents = Some(contents),
            Self::WalkProject { hover_contents } => *hover_contents = Some(contents),
        }
    }

    pub fn get_hover_contents(&self) -> Option<&HoverContents> {
        match self {
            Self::RagPrompt { hover_contents } => hover_contents.as_ref(),
            Self::QuickPrompt { hover_contents } => hover_contents.as_ref(),
            Self::WalkProject { hover_contents } => hover_contents.as_ref(),
        }
    }

    /// Gets trigger from GLOBAL_CONFIG, appends a whitespace
    pub fn trigger_string(&self) -> String {
        let actions_config = &GLOBAL_CONFIG.user_actions;
        match self {
            Self::QuickPrompt { .. } => actions_config.quick_prompt.to_owned(),
            Self::RagPrompt { .. } => actions_config.rag_prompt.to_owned(),
            Self::WalkProject { .. } => actions_config.walk_project.to_owned(),
        }
    }

    pub fn echo_content(&self) -> &str {
        match self {
            // Self::RagPrompt => "⧗",
            // Self::QuickPrompt => "⚑",
            // Self::WalkProject => "⧉",
            Self::RagPrompt { .. } => "RAGPROMPT",
            Self::QuickPrompt { .. } => "QUIK",
            Self::WalkProject { .. } => "WALK",
        }
    }

    pub fn user_input_diagnostic(&self) -> String {
        match self {
            Self::RagPrompt { .. } => "Goto Def to RAGPrompt agent",

            Self::QuickPrompt { .. } => "Goto Def to QuickPrompt agent",
            Self::WalkProject { .. } => "Goto Def to trigger a directory walk",
        }
        .to_string()
    }

    pub fn trigger_diagnostic(&self) -> String {
        match self {
            Self::RagPrompt { .. } => "",
            Self::QuickPrompt { .. } => "",
            Self::WalkProject { .. } => "",
        }
        .to_string()
    }

    /// denotes how many lines are supported for the given variant, as of right now all variants
    /// will only parse a single line
    fn user_input_size(&self) -> u32 {
        match self {
            _ => 1,
        }
    }

    /// Notification sent to client when action is being done
    pub fn doing_action_notification(&self) -> BufferOperation {
        match self {
            Self::QuickPrompt { .. } => {
                let work_done = WorkDoneProgressBegin {
                    title: "Quick Prompting Model".into(),
                    message: Some(String::from("Initializing")),
                    percentage: Some(0),
                    ..Default::default()
                };
                BufferOperation::WorkDone(WorkDoneProgress::Begin(work_done))
            }
            Self::RagPrompt { .. } => {
                let work_done = WorkDoneProgressBegin {
                    title: "RAG Prompting Model".into(),
                    message: Some(String::from("Initializing")),
                    percentage: Some(0),
                    ..Default::default()
                };
                BufferOperation::WorkDone(WorkDoneProgress::Begin(work_done))
            }
            Self::WalkProject { .. } => {
                let work_done = WorkDoneProgressBegin {
                    title: "Walking Project".into(),
                    message: Some(String::from("Initializing")),
                    percentage: Some(0),
                    ..Default::default()
                };
                BufferOperation::WorkDone(WorkDoneProgress::Begin(work_done))
            }
        }
    }

    /// returns (userinput, trigger)
    pub fn parse_for_user_input_and_trigger(
        &self,
        line_no: u32,
        text: &str,
    ) -> anyhow::Result<(Option<TextAndCharRange>, TextAndCharRange)> {
        let input_size = self.user_input_size();
        let line = text.lines().nth(line_no as usize).ok_or(anyhow!(
            "text has incorrect amount of lines. Expected at least {}",
            line_no + 1,
        ))?;

        if line.lines().count() != input_size as usize {
            return Err(anyhow!(
                "text has incorrect amount of lines. Expected {} Got {}",
                input_size,
                text.lines().count()
            ));
        }
        match self.user_input_size() {
            1 => {
                let initial_whitespace_len = line
                    .char_indices()
                    .find(|(_, ch)| !ch.is_whitespace())
                    .ok_or(anyhow!("the entire input string is whitespace"))?
                    .0;
                let chunks: Vec<&str> = line.split_whitespace().collect();
                debug!("chunks: {:?}", chunks);
                let trigger = chunks
                    .get(1)
                    .ok_or(anyhow!("not enough whitespace separated chunks in line"))?;
                if trigger != &self.echo_content() && trigger != &self.trigger_string() {
                    return Err(anyhow!(
                        "[{}] is not a valid trigger. Accepts [{}] and [{}]",
                        trigger,
                        self.echo_content(),
                        self.trigger_string()
                    ));
                }

                let uinput = match &self {
                    Self::QuickPrompt { .. } | Self::RagPrompt { .. } => {
                        let user_input = chunks[2..].join(" ");
                        let start =
                            (initial_whitespace_len + chunks[0].len() + trigger.len() + 2) as u32;
                        let end = start + user_input.len() as u32;
                        Some(TextAndCharRange {
                            text: user_input,
                            start,
                            end,
                        })
                    }
                    _ => None,
                };

                let start = (initial_whitespace_len + chunks[0].len() + 1) as u32;
                let end = start + trigger.len() as u32;

                let trig = TextAndCharRange {
                    text: trigger.to_string(),
                    start,
                    end,
                };

                return Ok((uinput, trig));
            }
            _ => unreachable!(),
        }
    }

    pub fn goto_definition_on_trigger_response(
        &self,
        request_id: RequestId,
    ) -> anyhow::Result<Option<BufferOperation>> {
        match self {
            Self::QuickPrompt { .. } | Self::RagPrompt { .. } => {
                let path = &GLOBAL_CONFIG.paths.conversation_file_path;
                let path_str = format!("file:///{}", path.display().to_string());
                Ok(Some(BufferOperation::GotoFile {
                    id: request_id,
                    response: GotoDefinitionResponse::Scalar(Location {
                        uri: Uri::from_str(&path_str)?,
                        range: Range::default(),
                    }),
                }))
            }
            Self::WalkProject { .. } => Ok(None),
        }
    }
}

mod tests {
    use crate::{error::init_test_tracing, state::burns::TextAndCharRange};

    use super::BurnActivation;

    #[test]
    fn correctly_parses_user_input() {
        init_test_tracing();
        let input = r#"
        this is not input
        // #$# this is user input 
        this is not "#;
        let output = BurnActivation::RagPrompt {
            hover_contents: None,
        }
        .parse_for_user_input_and_trigger(2, &input)
        .unwrap();
        let user_text_char = Some(TextAndCharRange {
            text: "this is user input".to_string(),
            start: 15,
            end: 33,
        });
        let trig_text_char = TextAndCharRange {
            text: "#$#".to_string(),
            start: 11,
            end: 14,
        };
        assert_eq!(output, (user_text_char, trig_text_char))
    }
}
