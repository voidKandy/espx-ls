use crate::helpers::{
    create_gotodef_params, into_lsp_request, test_buff_op_channel, test_state, TEST_TRACING,
};
use espx_lsp_server::{
    handle::{
        buffer_operations::{BufferOpChannelHandler, BufferOpChannelStatus, BufferOperation},
        requests::handle_goto_definition,
    },
    state::SharedState,
};
use lsp_types::{Position, Uri};
use std::{str::FromStr, sync::LazyLock};
use tracing_log::log::warn;

#[derive(Debug)]
enum BufferOpType {
    Diagnostics,
    WorkDone,
    ShowMessage,
    WorkspaceEdit,
    GotoFile,
    HoverResponse,
}

#[derive(Debug)]
struct BufferOpTypeVec(Vec<(BufferOpType, Option<usize>)>);

impl BufferOpType {
    fn same_as(&self, op: &BufferOperation) -> bool {
        match op {
            BufferOperation::WorkDone(_) => {
                if let Self::WorkDone = self {
                    return true;
                }
            }
            BufferOperation::WorkspaceEdit(_) => {
                if let Self::WorkspaceEdit = self {
                    return true;
                }
            }
            BufferOperation::Diagnostics(_) => {
                if let Self::Diagnostics = self {
                    return true;
                }
            }
            BufferOperation::ShowMessage(_) => {
                if let Self::ShowMessage = self {
                    return true;
                }
            }
            BufferOperation::GotoFile { .. } => {
                if let Self::GotoFile = self {
                    return true;
                }
            }
            BufferOperation::HoverResponse { .. } => {
                if let Self::HoverResponse = self {
                    return true;
                }
            }
        }
        false
    }
}

impl BufferOpTypeVec {
    fn validate_buffer_ops(&self, vec: Vec<BufferOperation>) -> bool {
        let mut total_gone_through = 0;
        for (typ, amt_opt) in &self.0 {
            match amt_opt {
                Some(amt) => {
                    let chunk = vec.iter().skip(total_gone_through).take(*amt);
                    total_gone_through += amt;
                    for op in chunk {
                        if !typ.same_as(op) {
                            return false;
                        }
                    }
                }
                None => {
                    let chunk = vec
                        .iter()
                        .skip(total_gone_through)
                        .take_while(|op| typ.same_as(op));

                    total_gone_through += chunk.count();
                }
            }
        }
        true
    }
}

#[tracing::instrument(name = "buffer op drain", skip_all)]
async fn poll_into_vec(handler: &mut BufferOpChannelHandler) -> Vec<BufferOperation> {
    let mut all = vec![];
    while let Some(status) = handler.receiver.recv().await {
        match status.unwrap() {
            BufferOpChannelStatus::Finished => break,
            BufferOpChannelStatus::Working(buffer_op) => {
                all.push(buffer_op);
            }
        }
    }
    all
}

async fn handler_tests_state() -> SharedState {
    let mut state = test_state(false).await;
    let mut update_state = || {
        let mut w = state.get_write().unwrap();
        let test_doc_1_uri = Uri::from_str("test_doc_1.rs").unwrap();
        let test_doc_1 = r#"use std::io::{self, Read};
// Comment without any command

// @_hey
fn main() {
    let mut raw = String::new();
    io::stdin()
        .read_to_string(&mut raw)
        .expect("failed to read io");
}

// +_
struct ToBePushed;
    "#;

        w.update_doc_and_agents_from_text(test_doc_1_uri, test_doc_1.to_string())
            .unwrap();
    };

    update_state();
    state
}

#[tokio::test]
async fn handles_push_gotodef_correctly() {
    LazyLock::force(&TEST_TRACING);
    let state = handler_tests_state().await;
    let mut buffer_op_channel = test_buff_op_channel();
    let test_doc_1_uri = Uri::from_str("test_doc_1.rs").unwrap();

    let push_pos = Position {
        line: 11,
        character: 4,
    };

    let push_request_params = create_gotodef_params(push_pos, test_doc_1_uri.clone());
    let req = into_lsp_request(push_request_params, 1, "textDocument/definition");
    handle_goto_definition(req, state, buffer_op_channel.sender.clone())
        .await
        .expect("failed handle goto def for prompt");
    buffer_op_channel.sender.send_finish().await.unwrap();

    let expected_ops = BufferOpTypeVec(vec![(BufferOpType::ShowMessage, Some(3))]);

    let all = poll_into_vec(&mut buffer_op_channel).await;
    warn!("All for push:\n{all:#?}");

    assert!(expected_ops.validate_buffer_ops(all));
}

#[tokio::test]
async fn handles_prompt_gotodef_correctly() {
    LazyLock::force(&TEST_TRACING);
    let state = handler_tests_state().await;
    let mut buffer_op_channel = test_buff_op_channel();
    let test_doc_1_uri = Uri::from_str("test_doc_1.rs").unwrap();

    let prompt_pos = Position {
        line: 3,
        character: 4,
    };

    let prompt_request_params = create_gotodef_params(prompt_pos, test_doc_1_uri.clone());

    let req = into_lsp_request(prompt_request_params, 1, "textDocument/definition");
    warn!("prompt req: {req:?}");
    handle_goto_definition(req, state.clone(), buffer_op_channel.sender.clone())
        .await
        .expect("failed handle goto def for prompt");
    buffer_op_channel.sender.send_finish().await.unwrap();

    let expected_ops = BufferOpTypeVec(vec![
        (BufferOpType::ShowMessage, Some(2)),
        (BufferOpType::WorkspaceEdit, Some(1)),
        (BufferOpType::WorkDone, None),
        (BufferOpType::ShowMessage, Some(1)),
    ]);
    let all = poll_into_vec(&mut buffer_op_channel).await;
    warn!("All for prompt:\n{all:#?}");

    assert!(expected_ops.validate_buffer_ops(all));
}
