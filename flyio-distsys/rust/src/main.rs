#![feature(hash_raw_entry)]

use std::{
    error::Error,
    time::{Duration, SystemTime},
};

use logger::Logger;
use message::{Message, MessageEnvelope, MessageReplyBuilder};
use mimalloc::MiMalloc;
use node::{Node, Topology};
use tokio::{runtime, time};
use tokio_stream::StreamExt;
use transport::{Transport, TransportWriter};

#[global_allocator]
static GLOBAL: MiMalloc = MiMalloc;

mod logger;
mod message;
mod node;
mod transport;

const DEBUG: bool = true;

fn main() -> Result<(), Box<dyn Error>> {
    let runtime = runtime::Builder::new_current_thread().enable_all().build()?;

    runtime.block_on(async {
        let mut logger = Logger::new();

        let node = Node::new();
        let topology = Topology::new();
        logger.log(format!("[{}] Starting!\n", node.id())).await;

        let (receiver, sender) = Transport::new().split();
        let mut ctx = NodeContext {
            node,
            topology,
            logger,
            sender,
        };

        let mut stream = receiver.recv_stream();
        let mut gossip_timer = time::interval(Duration::from_millis(100));

        loop {
            tokio::select! {
                _ = gossip_timer.tick() => {
                    let now = SystemTime::now();
                    let retry_messages = ctx.node.get_retryable_messages(now);
                    if !retry_messages.is_empty() {
                        let mut messages = Vec::new();
                        for (_, message, neighbor) in retry_messages {
                            let msg_id = ctx.node.get_next_msg_id();
                            messages.push(MessageEnvelope {
                                src: ctx.node.id_str().to_string(),
                                dest: neighbor.to_string(),
                                body: Message::Broadcast { msg_id, message },
                            });
                        }

                        if DEBUG {
                            ctx.logger.log(format!("[{}] Retrying messages: {}\n", ctx.node.id(), messages.len())).await;
                        }
                        send(&messages, &mut ctx).await;
                    }
                }
                result = stream.next() => {
                    let Some(result) = result else {
                        ctx.logger.log(format!("[{}] End of messages\n", ctx.node.id())).await;
                        break;
                    };
                    match result {
                        Ok(msg) => handle_message(msg, &mut ctx).await,
                        Err(e) => ctx.logger.log(format!("[{}] Error: {}\n", ctx.node.id(), e)).await,
                    };
                }
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
    sender: TransportWriter,
}

#[inline]
async fn handle_message(msg: MessageEnvelope, ctx: &mut NodeContext) {
    let logger = &mut ctx.logger;
    let node = &mut ctx.node;

    // if DEBUG {
    //     logger.log(format!("[{}] Got message: {:?}\n", node.id(), &msg)).await;
    // }

    let (metadata, message) = msg.split();
    let response_builder = MessageReplyBuilder::new(metadata);
    match message {
        Message::Init { msg_id, node_id, .. } => {
            ctx.topology.init(node_id.clone());
            node.init(node_id);
            let response = Message::InitOk {
                msg_id: node.get_next_msg_id(),
                in_reply_to: msg_id,
            };
            reply(response_builder, response, ctx).await;
        }
        Message::InitOk { .. } => {}
        Message::Echo { msg_id, echo } => {
            let response = Message::EchoOk {
                msg_id: node.get_next_msg_id(),
                in_reply_to: msg_id,
                echo: echo.clone(),
            };
            reply(response_builder, response, ctx).await;
        }
        Message::EchoOk { .. } => {}
        Message::Generate { msg_id } => {
            let response = Message::GenerateOk {
                msg_id: node.get_next_msg_id(),
                in_reply_to: msg_id,
                id: node.generate_unique_id(),
            };
            reply(response_builder, response, ctx).await;
        }
        Message::GenerateOk { .. } => {}
        Message::Topology { msg_id, topology } => {
            if DEBUG {
                logger
                    .log(format!(
                        "[{}] skipped neighbor - is src: \n{:?}\n",
                        node.id(),
                        &serde_json::to_string_pretty(&topology)
                    ))
                    .await;
            }

            ctx.topology.init_topology(topology);
            let response = Message::TopologyOk {
                msg_id: node.get_next_msg_id(),
                in_reply_to: msg_id,
            };
            reply(response_builder, response, ctx).await;
        }
        Message::TopologyOk { .. } => {}
        Message::Broadcast { msg_id, message } => {
            let is_new = node.add_message(message, &response_builder.request.src);
            if is_new {
                let neighbors = ctx.topology.get_my_neighbors();
                let mut messages = Vec::with_capacity(neighbors.len() + 1);
                let src = response_builder.request.src.to_string();
                messages.push(response_builder.build(Message::BroadcastOk {
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

                    let messages_to_broadcast = node.get_gossip_messages_for(neighbor);
                    if messages_to_broadcast.is_empty() {
                        if DEBUG {
                            logger
                                .log(format!(
                                    "[{}] skipped neighbor - already knows everything: {:?} {:?}\n",
                                    node.id(),
                                    neighbor,
                                    &message
                                ))
                                .await;
                        }
                        continue;
                    }

                    let now = SystemTime::now();
                    for message in messages_to_broadcast {
                        if node.message_is_pending(message, neighbor) {
                            if DEBUG {
                                logger
                                    .log(format!(
                                        "[{}] skipped neighbor - this message is pending: {:?} {:?}\n",
                                        node.id(),
                                        neighbor,
                                        &message
                                    ))
                                    .await;
                            }
                            continue;
                        }

                        let msg_id = node.get_next_msg_id();
                        messages.push(MessageEnvelope {
                            src: node.id_str().to_string(),
                            dest: neighbor.to_string(),
                            body: Message::Broadcast { msg_id, message },
                        });
                        node.add_pending_message(msg_id, message, now, neighbor);
                    }
                }
                send(&messages, ctx).await;
            } else {
                let response = Message::BroadcastOk {
                    msg_id: node.get_next_msg_id(),
                    in_reply_to: msg_id,
                };
                reply(response_builder, response, ctx).await;
            }
        }
        Message::BroadcastOk { in_reply_to, .. } => {
            if let Some((_, message, node_id)) = node.try_take_pending_message(in_reply_to) {
                node.node_acked_message(message, &node_id);
            }
        }
        Message::Read { msg_id } => {
            let response = Message::ReadOk {
                msg_id: node.get_next_msg_id(),
                in_reply_to: msg_id,
                messages: node.get_messages(),
            };
            reply(response_builder, response, ctx).await;
        }
        Message::ReadOk { .. } => {}
    };
}

#[inline]
async fn reply(builder: MessageReplyBuilder, body: Message, ctx: &mut NodeContext) {
    match ctx.sender.send(&[builder.build(body)]).await {
        Ok(_) => {}
        Err(e) => ctx.logger.log(format!("[{}] Error: {}\n", ctx.node.id(), e)).await,
    };
}

#[inline]
async fn send(bodies: &[MessageEnvelope], ctx: &mut NodeContext) {
    match ctx.sender.send(bodies).await {
        Ok(_) => {}
        Err(e) => ctx.logger.log(format!("[{}] Error: {}\n", ctx.node.id(), e)).await,
    };
}
