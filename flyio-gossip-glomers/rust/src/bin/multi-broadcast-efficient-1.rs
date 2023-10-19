#![feature(hash_raw_entry)]

use std::collections::{HashMap, HashSet};
use std::error::Error;
use std::time::SystemTime;

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
        messages_pending_by_value: HashSet::new(),
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
    messages_pending: FxHashMap<u64, (SystemTime, u64, String)>,
    messages_pending_by_value: HashSet<(u64, String)>,
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
    fn node_acked_message(&mut self, msg_id: u64) {
        let msg = self.messages_pending.remove(&msg_id);
        if let Some((_, message, node)) = msg {
            assert!(node.starts_with('n'));
            assert!(self.messages_pending_by_value.remove(&(message, node.clone())));
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
    }

    #[inline]
    fn add_pending_message(&mut self, msg_id: u64, message: u64, timestamp: SystemTime, node: &str) {
        assert!(self
            .messages_pending
            .insert(msg_id, (timestamp, message, node.to_string()))
            .is_none());

        self.messages_pending_by_value.insert((message, node.to_string()));
    }

    #[inline]
    fn get_messages(&self) -> Vec<u64> {
        self.messages.iter().copied().collect()
    }

    #[inline]
    fn get_retryable_messages(&self, now: SystemTime) -> Vec<(SystemTime, u64, String)> {
        self.messages_pending
            .iter()
            .filter(|&(_, (timestamp, _, _))| {
                let duration = now
                    .duration_since(*timestamp)
                    .expect("clock should always move forwards");
                duration.as_secs_f64() > 0.2
            })
            .map(|(_, v)| v)
            .cloned()
            .collect()
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
        let now = SystemTime::now();
        let retry_messages = self.get_retryable_messages(now);
        if !retry_messages.is_empty() {
            let mut messages = Vec::new();
            for (_, message, neighbor) in retry_messages {
                let msg_id = self.get_next_msg_id();
                messages.push(MessageEnvelope {
                    src: self.id_str().to_string(),
                    dest: neighbor.to_string(),
                    body: Message::Broadcast { msg_id, message },
                });
            }

            if DEBUG {
                ctx.logger
                    .log(format!("[{}] Retrying messages: {}\n", self.id(), messages.len()));
            }
            ctx.send(&messages, self).await;
        }
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
                let is_new = self.add_message(message, &response_builder.request.src);
                if is_new {
                    let src = response_builder.request.src.to_string();
                    let is_from_client = src.starts_with('c');
                    let message_cap = if is_from_client {
                        ctx.topology.node_count() + 1
                    } else {
                        1
                    };

                    let mut messages = Vec::with_capacity(message_cap);
                    messages.push(response_builder.build(Message::BroadcastOk {
                        msg_id: self.get_next_msg_id(),
                        in_reply_to: msg_id,
                    }));

                    if is_from_client {
                        let neighbors = ctx.topology.get_all_other_nodes();
                        let now = SystemTime::now();
                        for neighbor in &neighbors {
                            let msg_id = self.get_next_msg_id();
                            messages.push(MessageEnvelope {
                                src: self.id_str().to_string(),
                                dest: neighbor.to_string(),
                                body: Message::Broadcast { msg_id, message },
                            });
                            self.add_pending_message(msg_id, message, now, neighbor);
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
