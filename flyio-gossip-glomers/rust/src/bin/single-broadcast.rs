use std::collections::HashMap;
use std::error::Error;

use async_trait::async_trait;
use distsys::message::MessageReplyBuilder;
use distsys::{message::MessageEnvelope, node::NodeId};
use distsys::{MaelstromNode, RuntimeContext};
use serde::{Deserialize, Serialize};

fn main() -> Result<(), Box<dyn Error>> {
    let runtime = distsys::Runtime::new()?;

    let node = Node {
        id: NodeId::new(),
        next_msg_id: 0,
        messages: rustc_hash::FxHashSet::default(),
    };
    runtime.run(node);

    Ok(())
}

struct Node {
    id: NodeId,
    // Echo
    next_msg_id: u64,

    // Single node broadcast
    messages: rustc_hash::FxHashSet<u64>,
}

impl Node {
    #[inline]
    fn get_next_msg_id(&mut self) -> u64 {
        let id = self.next_msg_id;
        self.next_msg_id += 1;
        id
    }

    #[inline]
    fn add_message(&mut self, message: u64) {
        self.messages.insert(message);
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
                self.add_message(message);
                let response = Message::BroadcastOk {
                    msg_id: self.get_next_msg_id(),
                    in_reply_to: msg_id,
                };
                ctx.reply(response_builder, response, self).await;
            }
            Message::BroadcastOk { .. } => {}
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
