use anyhow::anyhow;
use espx_tui::{
    config::Config,
    handle::{self, buffer_operations::BufferOpChannelStatus},
    state::SharedState,
};
use lsp_server::Message;
use std::{
    path::Path,
    sync::{Arc, LazyLock},
    time::Duration,
};
use tokio::{
    io::{AsyncBufReadExt, BufReader},
    net::{UnixListener, UnixStream},
    sync::RwLock,
};
use tracing::warn;

const TUI_SOCKET_ADDR: &str = "/tmp/espx_tui_socket.sock";
const LSP_SOCKET_ADDR: &str = "/tmp/espx_lsp_socket.sock";
#[tokio::main]
async fn main() {
    LazyLock::force(&espx_tui::telemetry::TRACING);
    let config = Config::init();
    let state = SharedState::init(config).await.unwrap();
    if Path::new(TUI_SOCKET_ADDR).exists() {
        std::fs::remove_file(TUI_SOCKET_ADDR).unwrap();
    }

    let unix_listener = UnixListener::bind(TUI_SOCKET_ADDR).unwrap();
    warn!("created socket at: {TUI_SOCKET_ADDR}");

    #[allow(unused_assignments)]
    let mut unix_stream_opt = Option::<UnixStream>::None;

    loop {
        match UnixStream::connect(LSP_SOCKET_ADDR).await {
            Ok(stream) => {
                warn!("connected to lsp socket");
                unix_stream_opt = Some(stream);
                break;
            }

            Err(_) => {
                warn!("did not connect to socket at {LSP_SOCKET_ADDR}\nsleeping")
            }
        }

        tokio::time::sleep(Duration::from_millis(2000)).await;
    }

    let unix_stream = Arc::new(RwLock::new(
        unix_stream_opt.expect("Must have exited loop early"),
    ));

    loop {
        match unix_listener.accept().await {
            Ok((stream, _addr)) => {
                warn!("client: {_addr:?}");
                let unix_stream = Arc::clone(&unix_stream);
                let state = state.clone();
                tokio::spawn(async move {
                    let mut buf_reader = BufReader::new(stream);
                    let mut buf = String::new();
                    loop {
                        let bytes = buf_reader.read_line(&mut buf).await.unwrap();
                        if bytes == 0 {
                            warn!("Closed");
                            break;
                        }

                        if let Some(msg) = serde_json::from_str::<lsp_server::Message>(&buf).ok() {
                            warn!("Rcv: {msg:#?}");
                            match match msg {
                                Message::Notification(not) => {
                                    handle::notifications::handle_notification(not, state.clone())
                                        .await
                                }
                                Message::Request(req) => {
                                    handle::requests::handle_request(req, state.clone()).await
                                }
                                _ => Err(anyhow!("No handler for responses").into()),
                            } {
                                Ok(mut buffer_op_channel_handler) => {
                                    while let Some(status) =
                                        buffer_op_channel_handler.receiver.recv().await
                                    {
                                        match status.unwrap() {
                                            BufferOpChannelStatus::Finished => break,
                                            BufferOpChannelStatus::Working(buffer_op) => {
                                                buffer_op
                                                    .do_operation(Arc::clone(&unix_stream))
                                                    .await
                                                    .unwrap();
                                            }
                                        }
                                    }
                                }
                                Err(err) => {
                                    warn!("error in handler: {}", err);
                                }
                            }
                        }
                        buf.clear();
                    }
                });
            }
            Err(err) => {
                warn!("error connecting {err:#?}")
            }
        }
    }
}
