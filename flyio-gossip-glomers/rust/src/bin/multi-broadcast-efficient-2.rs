#![feature(hash_raw_entry)]

use std::collections::HashMap;
use std::error::Error;
use std::time::Duration;

use async_trait::async_trait;
use distsys::message::MessageReplyBuilder;
use distsys::{message::MessageEnvelope, node::NodeId};
use distsys::{MaelstromNode, RuntimeContext};
use rustc_hash::{FxHashMap, FxHashSet};
use serde::{Deserialize, Serialize};

const DEBUG: bool = false;

fn main() -> Result<(), Box<dyn Error>> {
    let runtime = distsys::Runtime::new_with_gossip_tick_interval(Duration::from_millis(500))?;

    let node = Node {
        id: NodeId::new(),
        next_msg_id: 0,
        messages: FxHashSet::default(),
        outboxes: HashMap::new(),
        outboxes_messages_pending: HashMap::new(),
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
    outboxes: HashMap<String, FxHashSet<u64>>,
    outboxes_messages_pending: HashMap<String, FxHashMap<u64, FxHashSet<u64>>>,
}

impl Node {
    #[inline]
    fn get_next_msg_id(&mut self) -> u64 {
        let id = self.next_msg_id;
        self.next_msg_id += 1;
        id
    }

    #[inline]
    fn add_message(&mut self, message: u64) -> bool {
        self.messages.insert(message)
    }

    #[inline]
    fn add_to_outbox(&mut self, message: u64, node: &str) {
        self.outboxes
            .raw_entry_mut()
            .from_key(node)
            .and_modify(|_, set| _ = set.insert(message))
            .or_insert_with(|| {
                let mut set = FxHashSet::default();
                set.insert(message);
                (node.to_string(), set)
            });
    }

    #[inline]
    fn get_outbox(&self, node: &str) -> Vec<u64> {
        let Some(outbox) = self.outboxes.get(node) else {
            return Vec::new();
        };

        match self.outboxes_messages_pending.get(node) {
            Some(pending) => {
                let mut messages = Vec::with_capacity(outbox.len());
                for pending_messages in pending.values() {
                    messages.extend(outbox.difference(pending_messages).copied());
                }

                messages
            }
            None => outbox.iter().copied().collect(),
        }
    }

    #[inline]
    fn sent_outbox_messages(&mut self, msg_id: u64, messages: &[u64], node: &str) {
        let (_, map) = self
            .outboxes_messages_pending
            .raw_entry_mut()
            .from_key(node)
            .or_insert_with(|| {
                let map = FxHashMap::default();
                (node.to_string(), map)
            });

        map.insert(msg_id, messages.iter().copied().collect());
    }

    #[inline]
    fn node_acked_messages(&mut self, msg_id: u64, node: &str) {
        let map = self.outboxes_messages_pending.get(node).unwrap();
        let messages = map.get(&msg_id).unwrap();
        let outbox = self.outboxes.get_mut(node).unwrap();
        outbox.retain(|m| !messages.contains(m));
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

    async fn handle_gossip_tick(&mut self, ctx: &mut RuntimeContext) {
        let neighbors = ctx.topology.get_all_other_nodes();
        let mut messages = Vec::with_capacity(neighbors.len());
        for neighbor in &neighbors {
            let outbox = self.get_outbox(neighbor);
            if !outbox.is_empty() {
                let msg_id = self.get_next_msg_id();
                self.sent_outbox_messages(msg_id, &outbox, neighbor);
                let message = Message::BroadcastMany {
                    msg_id,
                    messages: outbox,
                };
                let message = MessageEnvelope {
                    src: self.id_str().to_string(),
                    dest: neighbor.to_string(),
                    body: message,
                };
                messages.push(message);
            }
        }
        if DEBUG {
            ctx.logger
                .log(format!("[{}] Broadcasting messages: {}\n", self.id(), messages.len()));
        }
        ctx.send(&messages, self).await;
    }

    async fn handle_message(&mut self, msg: MessageEnvelope<Message>, ctx: &mut RuntimeContext) {
        let _logger = &mut ctx.logger;

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
                let is_new = self.add_message(message);

                if is_new {
                    let neighbors = ctx.topology.get_all_other_nodes();
                    for neighbor in &neighbors {
                        self.add_to_outbox(message, neighbor);
                    }
                }

                let response: Message = Message::BroadcastOk {
                    msg_id: self.get_next_msg_id(),
                    in_reply_to: msg_id,
                };
                ctx.reply(response_builder, response, self).await;
            }
            Message::BroadcastOk { .. } => {
                unreachable!("These messages should only be used for client->server")
            }
            Message::BroadcastMany { msg_id, messages } => {
                for &message in &messages {
                    self.add_message(message);
                }
                let response = Message::BroadcastManyOk {
                    msg_id: self.get_next_msg_id(),
                    in_reply_to: msg_id,
                };
                ctx.reply(response_builder, response, self).await;
            }
            Message::BroadcastManyOk { in_reply_to, .. } => {
                self.node_acked_messages(in_reply_to, &response_builder.request.src);
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
    BroadcastMany {
        msg_id: u64,
        messages: Vec<u64>,
    },
    BroadcastManyOk {
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
