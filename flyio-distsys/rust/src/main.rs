use std::error::Error;

use logger::Logger;
use mimalloc::MiMalloc;
use node::Node;
use protocol::{Protocol, ProtocolWriter};
use request::{Request, RequestEnvelope};
use response::Response;
use tokio::runtime;
use tokio_stream::StreamExt;

#[global_allocator]
static GLOBAL: MiMalloc = MiMalloc;

mod logger;
mod node;
mod protocol;
mod request;
mod response;

const DEBUG: bool = false;

fn main() -> Result<(), Box<dyn Error>> {
    let rt = runtime::Builder::new_current_thread().enable_all().build()?;

    rt.block_on(async {
        let mut logger = Logger::new();

        let mut node = Node::new();
        logger.log(format!("[{}] Starting!\n", node.id())).await;

        let (receiver, mut sender) = Protocol::new().split();

        let mut stream = receiver.recv_stream();
        while let Some(result) = stream.next().await {
            match result {
                Ok(msg) => handle_message(msg, &mut node, &mut logger, &mut sender).await,
                Err(e) => logger.log(format!("[{}] Error: {}\n", node.id(), e)).await,
            }
        }

        logger.log(format!("[{}] Exiting...\n", node.id())).await;
    });

    Ok(())
}

#[inline]
async fn handle_message(msg: RequestEnvelope, node: &mut Node, logger: &mut Logger, sender: &mut ProtocolWriter) {
    if DEBUG {
        logger.log(format!("[{}] Got message: {:?}\n", node.id(), &msg)).await;
    }

    let response = match &msg.body {
        Request::Init { msg_id, node_id, .. } => {
            node.init(node_id);
            Some(Response::InitOk {
                msg_id: node.get_next_msg_id(),
                in_reply_to: *msg_id,
            })
        }
        Request::Echo { msg_id, echo } => Some(Response::EchoOk {
            msg_id: node.get_next_msg_id(),
            in_reply_to: *msg_id,
            echo: echo.clone(),
        }),
        Request::Generate { msg_id } => Some(Response::GenerateOk {
            msg_id: node.get_next_msg_id(),
            in_reply_to: *msg_id,
            id: node.generate_unique_id(),
        }),
    };

    if let Some(body) = response {
        match sender.send(msg.to_response(body)).await {
            Ok(_) => {}
            Err(e) => logger.log(format!("[{}] Error: {}\n", node.id(), e)).await,
        };
    }
}
