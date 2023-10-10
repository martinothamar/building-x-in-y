#![feature(hash_raw_entry)]

use std::collections::HashMap;
use std::error::Error;

use async_trait::async_trait;
use distsys::message::MessageReplyBuilder;
use distsys::{message::MessageEnvelope, node::NodeId};
use distsys::{MaelstromNode, RuntimeContext};
use rustc_hash::{FxHashMap, FxHashSet};
use serde::{Deserialize, Serialize};

const DEBUG: bool = false;

fn main() -> Result<(), Box<dyn Error>> {
    let runtime = distsys::Runtime::new()?;

    let node = Node {
        id: NodeId::new(),
        next_msg_id: 0,
        messages: FxHashSet::default(),
        messages_by_node: HashMap::new(),
        messages_pending: FxHashMap::default(),
    };
    runtime.run(node);

    Ok(())
}

struct Node {
    id: NodeId,
    // Echo
    next_msg_id: u64,

    // Multi node broadcast
    messages: FxHashSet<u64>,
    messages_by_node: HashMap<String, FxHashSet<u64>>,
    messages_pending: FxHashMap<u64, (u64, String)>,
}

impl Node {
    #[inline]
    fn get_next_msg_id(&mut self) -> u64 {
        let id = self.next_msg_id;
        self.next_msg_id += 1;
        id
    }

    #[inline]
    fn add_message(&mut self, message: u64, from: &str) -> bool {
        if from.starts_with('n') {
            self.messages_by_node
                .raw_entry_mut()
                .from_key(from)
                .and_modify(|_, set| _ = set.insert(message))
                .or_insert_with(|| {
                    let mut set = FxHashSet::default();
                    set.insert(message);
                    (from.to_string(), set)
                });
        }
        self.messages.insert(message)
    }

    #[inline]
    fn get_gossip_messages_for(&self, node: &str) -> Vec<u64> {
        let node_messages = self.messages_by_node.get(node);
        match node_messages {
            Some(set) => self.messages.difference(set).copied().collect(),
            None => self.messages.iter().copied().collect(),
        }
    }

    #[inline]
    fn node_acked_message(&mut self, msg_id: u64) {
        let (message, node) = self.messages_pending.remove(&msg_id).unwrap();
        assert!(node.starts_with('n'));
        self.messages_by_node
            .raw_entry_mut()
            .from_key(&node)
            .and_modify(|_, set| _ = set.insert(message))
            .or_insert_with(|| {
                let mut set = FxHashSet::default();
                set.insert(message);
                (node, set)
            });
    }

    #[inline]
    fn add_pending_message(&mut self, msg_id: u64, message: u64, node: &str) {
        assert!(node.starts_with('n'));
        assert!(self
            .messages_pending
            .insert(msg_id, (message, node.to_string()))
            .is_none());
    }

    #[inline]
    fn get_messages(&self) -> Vec<u64> {
        self.messages.iter().copied().collect()
    }
}

#[async_trait]
impl MaelstromNode<Message> for Node {
    fn init(&mut self, id: String) {
        self.id.init(id);
    }

    fn id(&self) -> &NodeId {
        &self.id
    }

    fn id_str(&self) -> &str {
        self.id.0.as_ref().unwrap()
    }

    async fn handle_gossip_tick(&mut self, _ctx: &mut RuntimeContext) {}

    async fn handle_message(&mut self, msg: MessageEnvelope<Message>, ctx: &mut RuntimeContext) {
        let logger = &mut ctx.logger;

        let (metadata, message) = msg.split();
        let response_builder = MessageReplyBuilder::new(metadata);
        match message {
            Message::Init { msg_id, node_id, .. } => {
                ctx.topology.init(node_id.clone());
                self.init(node_id);
                let response = Message::InitOk {
                    msg_id: self.get_next_msg_id(),
                    in_reply_to: msg_id,
                };
                ctx.reply(response_builder, response, self).await;
            }
            Message::InitOk { .. } => {}
            Message::Topology { msg_id, topology } => {
                ctx.topology.init_topology(topology);
                let response = Message::TopologyOk {
                    msg_id: self.get_next_msg_id(),
                    in_reply_to: msg_id,
                };
                ctx.reply(response_builder, response, self).await;
            }
            Message::TopologyOk { .. } => {}
            Message::Broadcast { msg_id, message } => {
                let is_new = self.add_message(message, &response_builder.request.src);
                if is_new {
                    let neighbors = ctx.topology.get_my_neighbors();
                    let mut messages = Vec::with_capacity(neighbors.len() + 1);
                    let src = response_builder.request.src.to_string();
                    messages.push(response_builder.build(Message::BroadcastOk {
                        msg_id: self.get_next_msg_id(),
                        in_reply_to: msg_id,
                    }));

                    let broadcasters_neighbors = ctx.topology.get_neighbors(&src);

                    for neighbor in neighbors {
                        if src.eq(neighbor) {
                            if DEBUG {
                                logger.log(format!("[{}] skipped neighbor - is src: {:?}\n", self.id(), &src));
                            }
                            continue; // Don't broadcast back to sender
                        }
                        if broadcasters_neighbors.contains(neighbor) {
                            if DEBUG {
                                logger.log(format!(
                                    "[{}] skipped neighbor - src also has this neighbor: {:?} {:?}\n",
                                    self.id(),
                                    &src,
                                    neighbor
                                ));
                            }
                            // Current sender also has this neighbor, so they've already got it
                            // NOTE: haven't actually seen this happen using the topologies
                            // given by maelstrom...
                            continue;
                        }

                        let messages_to_broadcast = self.get_gossip_messages_for(neighbor);
                        if messages_to_broadcast.is_empty() {
                            if DEBUG {
                                logger.log(format!(
                                    "[{}] skipped neighbor - already knows everything: {:?} {:?}\n",
                                    self.id(),
                                    neighbor,
                                    &message
                                ));
                            }
                            continue;
                        }

                        for message in messages_to_broadcast {
                            let msg_id = self.get_next_msg_id();
                            messages.push(MessageEnvelope {
                                src: self.id_str().to_string(),
                                dest: neighbor.to_string(),
                                body: Message::Broadcast { msg_id, message },
                            });
                            self.add_pending_message(msg_id, message, neighbor);
                        }
                    }
                    ctx.send(&messages, self).await;
                } else {
                    let response = Message::BroadcastOk {
                        msg_id: self.get_next_msg_id(),
                        in_reply_to: msg_id,
                    };
                    ctx.reply(response_builder, response, self).await;
                }
            }
            Message::BroadcastOk { in_reply_to, .. } => {
                self.node_acked_message(in_reply_to);
            }
            Message::Read { msg_id } => {
                let messages = self.get_messages();
                let response = Message::ReadOk {
                    msg_id: self.get_next_msg_id(),
                    in_reply_to: msg_id,
                    messages,
                };
                ctx.reply(response_builder, response, self).await;
            }
            Message::ReadOk { .. } => {}
        };
    }
}

#[derive(Serialize, Deserialize, Debug, Clone)]
#[serde(tag = "type", rename_all = "snake_case")]
#[allow(clippy::enum_variant_names)]
pub enum Message {
    Init {
        msg_id: u64,
        node_id: String,
        node_ids: Vec<String>,
    },
    InitOk {
        msg_id: u64,
        in_reply_to: u64,
    },
    Topology {
        msg_id: u64,
        topology: HashMap<String, Vec<String>>,
    },
    TopologyOk {
        msg_id: u64,
        in_reply_to: u64,
    },
    Broadcast {
        msg_id: u64,
        message: u64,
    },
    BroadcastOk {
        msg_id: u64,
        in_reply_to: u64,
    },
    Read {
        msg_id: u64,
    },
    ReadOk {
        msg_id: u64,
        in_reply_to: u64,
        messages: Vec<u64>,
    },
}
