use std::error::Error;

use logger::Logger;
use mimalloc::MiMalloc;
use node::{Node, Topology};
use protocol::{Protocol, ProtocolWriter};
use request::{Request, RequestEnvelope};
use response::{Response, ResponseBuilder, ResponseEnvelope};
use tokio::runtime;
use tokio_stream::StreamExt;

#[global_allocator]
static GLOBAL: MiMalloc = MiMalloc;

mod logger;
mod node;
mod protocol;
mod request;
mod response;

const DEBUG: bool = true;

fn main() -> Result<(), Box<dyn Error>> {
    let runtime = runtime::Builder::new_current_thread().enable_all().build()?;

    runtime.block_on(async {
        let mut logger = Logger::new();

        let node = Node::new();
        let topology = Topology::new();
        logger.log(format!("[{}] Starting!\n", node.id())).await;

        let (receiver, sender) = Protocol::new().split();
        let mut ctx = NodeContext {
            node,
            topology,
            logger,
            sender,
        };

        let mut stream = receiver.recv_stream();
        while let Some(result) = stream.next().await {
            match result {
                Ok(msg) => handle_message(msg, &mut ctx).await,
                Err(e) => ctx.logger.log(format!("[{}] Error: {}\n", ctx.node.id(), e)).await,
            }
        }

        ctx.logger.log(format!("[{}] Exiting...\n", ctx.node.id())).await;
    });

    Ok(())
}

struct NodeContext {
    node: Node,
    topology: Topology,
    logger: Logger,
    sender: ProtocolWriter,
}

#[inline]
async fn handle_message(msg: RequestEnvelope, ctx: &mut NodeContext) {
    let logger = &mut ctx.logger;
    let node = &mut ctx.node;

    // if DEBUG {
    //     logger.log(format!("[{}] Got message: {:?}\n", node.id(), &msg)).await;
    // }

    let (metadata, request) = msg.split();
    let response_builder = ResponseBuilder::new(metadata);
    match request {
        Request::Init { msg_id, node_id, .. } => {
            ctx.topology.init(node_id.clone());
            node.init(node_id);
            let response = Response::InitOk {
                msg_id: node.get_next_msg_id(),
                in_reply_to: msg_id,
            };
            reply(response_builder, response, ctx).await;
        }
        Request::Echo { msg_id, echo } => {
            let response = Response::EchoOk {
                msg_id: node.get_next_msg_id(),
                in_reply_to: msg_id,
                echo: echo.clone(),
            };
            reply(response_builder, response, ctx).await;
        }
        Request::Generate { msg_id } => {
            let response = Response::GenerateOk {
                msg_id: node.get_next_msg_id(),
                in_reply_to: msg_id,
                id: node.generate_unique_id(),
            };
            reply(response_builder, response, ctx).await;
        }
        Request::Topology { msg_id, topology } => {
            ctx.topology.init_topology(topology);
            let response = Response::TopologyOk {
                msg_id: node.get_next_msg_id(),
                in_reply_to: msg_id,
            };
            reply(response_builder, response, ctx).await;
        }
        Request::Broadcast { msg_id, message } => {
            let is_new = node.add_message(message);
            if is_new {
                let neighbors = ctx.topology.get_my_neighbors();
                let mut messages = Vec::with_capacity(neighbors.len() + 1);
                let src = response_builder.request.src.to_string();
                messages.push(response_builder.build(Response::BroadcastOk {
                    msg_id: node.get_next_msg_id(),
                    in_reply_to: msg_id,
                }));

                let broadcasters_neighbors = ctx.topology.get_neighbors(&src);

                for neighbor in neighbors {
                    if src.eq(neighbor) {
                        if DEBUG {
                            logger
                                .log(format!("[{}] skipped neighbor - is src: {:?}\n", node.id(), &src))
                                .await;
                        }
                        continue; // Don't broadcast back to sender
                    }
                    if broadcasters_neighbors.contains(neighbor) {
                        if DEBUG {
                            logger
                                .log(format!(
                                    "[{}] skipped neighbor - src also has this neighbor: {:?} {:?}\n",
                                    node.id(),
                                    &src,
                                    neighbor
                                ))
                                .await;
                        }
                        continue; // Current sender also has this neighbor, so they've already got it
                    }
                    messages.push(ResponseEnvelope {
                        src: node.id_str().to_string(),
                        dest: neighbor.to_string(),
                        body: Response::Broadcast {
                            msg_id: node.get_next_msg_id(),
                            message,
                        },
                    });
                }
                send(&messages, ctx).await;
            } else {
                let response = Response::BroadcastOk {
                    msg_id: node.get_next_msg_id(),
                    in_reply_to: msg_id,
                };
                reply(response_builder, response, ctx).await;
            }
        }
        Request::Read { msg_id } => {
            let response = Response::ReadOk {
                msg_id: node.get_next_msg_id(),
                in_reply_to: msg_id,
                messages: node.get_messages(),
            };
            reply(response_builder, response, ctx).await;
        }
        Request::BroadcastOk { .. } => {}
    };
}

#[inline]
async fn reply(builder: ResponseBuilder, body: Response, ctx: &mut NodeContext) {
    match ctx.sender.send(&[builder.build(body)]).await {
        Ok(_) => {}
        Err(e) => ctx.logger.log(format!("[{}] Error: {}\n", ctx.node.id(), e)).await,
    };
}

#[inline]
async fn send(bodies: &[ResponseEnvelope], ctx: &mut NodeContext) {
    match ctx.sender.send(bodies).await {
        Ok(_) => {}
        Err(e) => ctx.logger.log(format!("[{}] Error: {}\n", ctx.node.id(), e)).await,
    };
}
