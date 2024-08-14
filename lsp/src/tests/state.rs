use std::str::FromStr;

use lsp_types::{Position, Range, TextDocumentContentChangeEvent, Uri};

use crate::state::{
    burns::{Burn, SingleLineActivation, SingleLineVariant},
    GlobalState,
};

use super::test_config;

async fn test_state() -> GlobalState {
    GlobalState::init(&test_config()).await.unwrap()
}

#[tokio::test]
async fn update_burns_and_doc_from_lsp_change_notification_works() {
    // super::init_test_tracing();
    let mut state = setup().await;
    let change = TextDocumentContentChangeEvent {
        range: Some(lsp_types::Range {
            start: lsp_types::Position {
                line: 4,
                character: 0,
            },
            end: lsp_types::Position {
                line: 5,
                character: 0,
            },
        }),
        text: String::from("#$ the burn has changed\n"),
        range_length: None,
    };

    let uri = Uri::from_str("file:///tmp/foo").unwrap();
    // s.burns.take_burns_on_doc(&uri).unwrap();

    let act = SingleLineActivation::new(
        SingleLineVariant::QuickPrompt,
        "#$",
        Range {
            start: Position {
                line: 4,
                character: 0,
            },
            end: Position {
                line: 4,
                character: 2,
            },
        },
    );

    let expted_burn = Burn::from(act);

    state
        .update_burns_from_lsp_change_notification(&change, uri.clone())
        .unwrap();
    state
        .update_doc_from_lsp_change_notification(&change, uri.clone())
        .unwrap();

    let b = state.store.burns.read_burns_on_doc(&uri).unwrap();
    assert!(b[0].activation.matches_variant(&expted_burn.activation));
    assert_eq!(b[0].activation.range(), expted_burn.activation.range());
}

async fn setup() -> GlobalState {
    let mut state = test_state().await;

    let uri = Uri::from_str("file:///tmp/foo").unwrap();
    let text = r#"
     This is chunk 1 of foo


     #$ There is a burn here















     .............
     This is chunk 2 of foo
     ...............
     "#;

    state.store.update_doc(text, uri.clone());
    for burn in Burn::all_in_text(text) {
        state.store.burns.insert_burn(uri.clone(), burn);
    }

    let uri = Uri::from_str("file:///tmp/bar").unwrap();
    let text = r#"
     This is chunk 1 of bar


     @@















     .............
     This is chunk 2 of bar 
     ...............
     "#;
    state.store.update_doc(text, uri.clone());
    for burn in Burn::all_in_text(text) {
        state.store.burns.insert_burn(uri.clone(), burn);
    }

    let uri = Uri::from_str("file:///tmp/baz").unwrap();
    let text = "baz is small";
    state.store.update_doc(text, uri.clone());
    for burn in Burn::all_in_text(text) {
        state.store.burns.insert_burn(uri.clone(), burn);
    }

    state
}
