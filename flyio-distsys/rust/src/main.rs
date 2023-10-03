use std::error::Error;

use protocol::Protocol;
use request::{NodeId, Request};
use response::Response;
use tokio::{
    io::{self, AsyncWriteExt, BufWriter},
    runtime,
};
use tokio_stream::StreamExt;

mod protocol;
mod request;
mod response;

fn main() -> Result<(), Box<dyn Error>> {
    let rt = runtime::Builder::new_current_thread().enable_all().build()?;

    rt.block_on(async {
        let mut logger = Logger::new();

        let mut current_node_id = NodeId::new();
        logger.log(format!("[{}] Starting!\n", &current_node_id)).await;

        let (receiver, mut sender) = Protocol::new().split();

        let mut stream = receiver.recv_stream();
        while let Some(result) = stream.next().await {
            match result {
                Ok(msg) => {
                    // Handle message
                    logger
                        .log(format!("[{}] Got message: {:?}\n", &current_node_id, &msg))
                        .await;

                    let response = match &msg.body {
                        Request::Init { msg_id, node_id, .. } => {
                            current_node_id.init(node_id);
                            Some(Response::InitOk {
                                msg_id: sender.get_next_msg_id(),
                                in_reply_to: *msg_id,
                            })
                        }
                        Request::Echo { msg_id, echo } => Some(Response::EchoOk {
                            msg_id: sender.get_next_msg_id(),
                            in_reply_to: *msg_id,
                            echo: echo.clone(),
                        }),
                        Request::Read { .. } => todo!(),
                    };

                    if let Some(body) = response {
                        match sender.send(msg.to_response(body)).await {
                            Ok(_) => {}
                            Err(e) => logger.log(format!("[{}] Error: {}\n", &current_node_id, e)).await,
                        };
                    }
                }
                Err(e) => logger.log(format!("[{}] Error: {}\n", &current_node_id, e)).await,
            }
        }

        logger.log(format!("[{}] Exiting...\n", &current_node_id)).await;
    });

    Ok(())
}

struct Logger {
    inner: BufWriter<io::Stderr>,
}

impl Logger {
    fn new() -> Self {
        let stderr: io::Stderr = io::stderr();
        Self {
            inner: BufWriter::new(stderr),
        }
    }

    async fn log<T: AsRef<[u8]>>(&mut self, data: T) {
        let buf = data.as_ref();
        self.inner
            .write_all(buf)
            .await
            .unwrap_or_else(|_| std::process::exit(420));
        self.inner.flush().await.unwrap_or_else(|_| std::process::exit(420));
    }
}
