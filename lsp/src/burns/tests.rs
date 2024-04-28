use lsp_types::{Position, Range};
use super::{actions::{ActionBurn, ActionType}, Burn, EchoBurn};

pub fn mock_burns() -> Vec<Burn> {
    let mut all_burns = vec![];
   all_burns.push(EchoBurn {
        content: "@".to_string(),
        hover_contents: lsp_types::HoverContents::Scalar(
            lsp_types::MarkedString::String("This is test content for @".to_string())
        ),
        range: Range {
            start: Position { line: 1, character: 1 },
            end: Position { line: 1, character: 3 } 
        }
    }.into());

   all_burns.push(EchoBurn {
        content: "&".to_string(),
        hover_contents: lsp_types::HoverContents::Scalar(
            lsp_types::MarkedString::String("This is test content for &".to_string())
        ),
        range: Range {
            start: Position { line: 2, character: 1 },
            end: Position { line: 2, character: 3 } 
        }
    }.into());

   all_burns.push(EchoBurn {
        content: "%".to_string(),
        hover_contents: lsp_types::HoverContents::Scalar(
            lsp_types::MarkedString::String("This is test content for %".to_string())
        ),
        range: Range {
            start: Position { line: 3, character: 1 },
            end: Position { line: 3, character: 3 } 
        }
    }.into());

   all_burns.push(ActionBurn {
        typ: ActionType::IoPrompt,
        range: Range {
            start: Position { line: 4, character: 1 },
            end: Position { line: 4, character: 5 } 
        },
        user_input: Some("hey".to_string()),
        replacement_text: String::new(),
    }.into());

   all_burns.push(ActionBurn {
        typ: ActionType::WalkProject,
        range: Range {
            start: Position { line: 5, character: 2 },
            end: Position { line: 5, character: 3 } 
        },
        user_input: None,
        replacement_text: String::new(),
    }.into());

    all_burns
}

#[test]
fn parse_for_actions_returns_actions_correctly() {
    let input = r#"
        #$ Hello
        Blah blah blah
        this is not an action
        @@
        more stuff that isn't an action
    "#;
    let actions = ActionType::parse_for_actions(input);
    assert_eq!(2, actions.len());

    let expected_start = Position {
        line: 1,
        character: 11,
    };
    let expected_end = Position {
        line: 1,
        character: 16,
    };
    assert_eq!(actions[0].range , Range {
        start: expected_start,
        end: expected_end,
    }  );

    let expected_start = Position {
        line: 4,
        character: 8,
    };
    let expected_end = Position {
        line: 4,
        character: 10,
    };
    assert_eq!(actions[1].range , Range {
        start: expected_start,
        end: expected_end,
    }  );
}




