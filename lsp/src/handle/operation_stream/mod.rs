pub mod error;

pub use error::*;
use futures::{self, Stream};

use super::BufferOperation;

pub type BufferOpStream = Box<dyn Stream<Item = BufferOpStreamResult<BufferOpStreamStatus>>>;

pub type BufferOpStreamReceiver =
    tokio::sync::mpsc::Receiver<BufferOpStreamResult<BufferOpStreamStatus>>;
#[derive(Clone)]
pub struct BufferOpStreamSender(
    tokio::sync::mpsc::Sender<BufferOpStreamResult<BufferOpStreamStatus>>,
);

pub struct BufferOpStreamHandler {
    stream: Option<BufferOpStream>,
    pub sender: BufferOpStreamSender,
    pub receiver: BufferOpStreamReceiver,
}

#[derive(Debug)]
pub enum BufferOpStreamStatus {
    Working(BufferOperation),
    Finished,
}

impl From<BufferOperation> for BufferOpStreamStatus {
    fn from(value: BufferOperation) -> Self {
        Self::Working(value)
    }
}

impl BufferOpStreamHandler {
    pub fn new() -> Self {
        let channel = tokio::sync::mpsc::channel::<BufferOpStreamResult<BufferOpStreamStatus>>(5);
        Self {
            stream: None,
            sender: BufferOpStreamSender(channel.0),
            receiver: channel.1,
        }
    }
}

impl BufferOpStreamSender {
    pub async fn send_operation(&mut self, op: BufferOperation) -> BufferOpStreamResult<()> {
        Ok(self.0.send(Ok(op.into())).await?)
    }
    pub async fn send_finish(&self) -> BufferOpStreamResult<()> {
        Ok(self.0.send(Ok(BufferOpStreamStatus::Finished)).await?)
    }
}

mod tests {
    #[allow(unused)]
    use super::*;
    #[tokio::test]
    async fn buffer_op_stream_works() {
        let mut ops_stream_handler = BufferOpStreamHandler::new();

        let _: tokio::task::JoinHandle<BufferOpStreamResult<()>> = tokio::spawn(async move {
            for _ in 0..5 {
                ops_stream_handler
                    .sender
                    .send_operation(BufferOperation::ShowMessage(lsp_types::ShowMessageParams {
                        typ: lsp_types::MessageType::INFO,
                        message: "".to_owned(),
                    }))
                    .await
                    .unwrap();
            }
            ops_stream_handler.sender.send_finish().await.unwrap();
            Ok(())
        });

        let mut counter = 0;
        while let Ok(BufferOpStreamStatus::Working(buffer_op)) =
            ops_stream_handler.receiver.recv().await.unwrap()
        {
            counter += 1;
            println!("{:?}", buffer_op)
        }
        assert_eq!(5, counter);
    }
}
