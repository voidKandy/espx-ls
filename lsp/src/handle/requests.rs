use super::{
    buffer_operations::{BufferOpChannelHandler, BufferOpChannelSender, BufferOperation},
    diagnostics::LspDiagnostic,
    error::HandleResult,
};
use crate::{
    handle::BufferOpChannelJoinHandle,
    state::{
        burns::{Activation, Burn},
        espx::AgentID,
        SharedGlobalState,
    },
};
use lsp_server::Request;
use lsp_types::{DocumentDiagnosticParams, GotoDefinitionParams, HoverParams, Position};
use tracing::{debug, warn};

#[tracing::instrument(name = "handle request", skip_all)]
pub async fn handle_request(
    req: Request,
    state: SharedGlobalState,
) -> HandleResult<BufferOpChannelHandler> {
    let handle = BufferOpChannelHandler::new();
    let task_sender = handle.sender.clone();
    let _: BufferOpChannelJoinHandle = tokio::spawn(async move {
        match match req.method.as_str() {
            "textDocument/definition" => {
                handle_goto_definition(req, state, task_sender.clone()).await
            }
            "textDocument/hover" => handle_hover(req, state, task_sender.clone()).await,
            "textDocument/diagnostic" => handle_diagnostics(req, state, task_sender.clone()).await,
            "shutdown" => handle_shutdown(state, task_sender.clone()).await,
            _ => {
                warn!("unhandled request method: {}", req.method);
                Ok(())
            }
        } {
            Ok(_) => task_sender.send_finish().await.map_err(|err| err.into()),
            Err(err) => return Err(err),
        }
    });
    return Ok(handle);
}

#[tracing::instrument(name = "goto def", skip_all)]
async fn handle_goto_definition(
    req: Request,
    mut state: SharedGlobalState,
    mut sender: BufferOpChannelSender,
) -> HandleResult<()> {
    let params = serde_json::from_value::<GotoDefinitionParams>(req.params)?;

    let actual_pos = Position {
        line: params.text_document_position_params.position.line,
        character: params.text_document_position_params.position.character + 1,
    };
    let uri = params.text_document_position_params.text_document.uri;

    let mut w = state.get_write()?;

    debug!("current burns in store: {:?}", w.store.burns);

    if let Some(mut burn) = w.store.burns.take_burn(&uri, actual_pos.line) {
        if let Activation::Single(_) = burn.activation {
            burn.activate_with_agent(
                uri.clone(),
                Some(req.id),
                Some(actual_pos),
                &mut sender,
                &mut w,
            )
            .await?;
        } else {
            warn!("No multi line burns have any reason to have positional activation");
        }
        debug!("finished activating burn");
        w.store.burns.insert_burn(uri, burn);
    }

    debug!("goto def returned ok");

    Ok(())
}

#[tracing::instrument(name = "hover", skip_all)]
async fn handle_hover(
    req: Request,
    state: SharedGlobalState,
    mut sender: BufferOpChannelSender,
) -> HandleResult<()> {
    let params = serde_json::from_value::<HoverParams>(req.params)?;
    let uri = params.text_document_position_params.text_document.uri;
    let position = params.text_document_position_params.position;

    debug!(
        "got hover request on doc: {:?} as position: {:?}",
        uri.as_str(),
        position
    );

    let r = state.get_read()?;
    if let Some(burns_on_doc) = r.store.burns.read_burns_on_doc(&uri) {
        if let Some(burn) = burns_on_doc
            .iter()
            .find(|b| b.activation.is_in_position(&position))
        {
            if let Some(contents) = &burn.hover_contents {
                sender
                    .send_operation(BufferOperation::HoverResponse {
                        contents: contents.clone(),
                        id: req.id,
                    })
                    .await?;
            }
        }
    }

    Ok(())
}

async fn handle_diagnostics(
    req: Request,
    mut state: SharedGlobalState,
    mut sender: BufferOpChannelSender,
) -> HandleResult<()> {
    let params: DocumentDiagnosticParams =
        serde_json::from_value::<DocumentDiagnosticParams>(req.params)?;
    let mut w = state.get_write()?;
    sender
        .send_operation(
            LspDiagnostic::diagnose_document(params.text_document.uri, &mut w.store)?.into(),
        )
        .await?;
    Ok(())
}

async fn handle_shutdown(
    mut state: SharedGlobalState,
    mut sender: BufferOpChannelSender,
) -> HandleResult<()> {
    warn!("shutting down server");
    sender.start_work_done(Some("Shutting down server")).await?;
    let mut w = state.get_write()?;
    if let Some(mut db) = w.store.db.take() {
        sender
            .send_work_done_report(Some("Database present, Saving state..."), None)
            .await?;
        warn!("saving current state to database");

        match w.store.try_update_database().await {
            Ok(_) => debug!("succesfully updated database"),
            Err(err) => warn!("problem updating database: {:?}", err),
        };
        sender
            .send_work_done_report(Some("Finished saving state, shutting down database"), None)
            .await?;

        warn!("shutting down database");
    }
    sender
        .send_work_done_end(Some("Finished Server shutdown"))
        .await?;
    Ok(())
}
