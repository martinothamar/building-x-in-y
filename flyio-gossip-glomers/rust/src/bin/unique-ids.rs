use std::error::Error;
use std::time::SystemTime;

use async_trait::async_trait;
use distsys::message::MessageReplyBuilder;
use distsys::{message::MessageEnvelope, node::NodeId};
use distsys::{MaelstromNode, RuntimeContext};
use rand::SeedableRng;
use serde::{Deserialize, Serialize};
use ulid::serde::ulid_as_uuid;

fn main() -> Result<(), Box<dyn Error>> {
    let runtime = distsys::Runtime::new()?;

    let node = Node {
        id: NodeId::new(),
        next_msg_id: 0,
        unique_id_generator: ulid::Generator::new(),
        prng: rand::rngs::SmallRng::from_entropy(),
    };
    runtime.run(node);

    Ok(())
}

struct Node {
    id: NodeId,
    // Echo
    next_msg_id: u64,

    // Unique ids
    unique_id_generator: ulid::Generator,
    prng: rand::rngs::SmallRng,
}

impl Node {
    #[inline]
    fn get_next_msg_id(&mut self) -> u64 {
        let id = self.next_msg_id;
        self.next_msg_id += 1;
        id
    }

    #[inline]
    fn generate_unique_id(&mut self) -> ulid::Ulid {
        let now = SystemTime::now();
        self.unique_id_generator
            .generate_from_datetime_with_source(now, &mut self.prng)
            .expect("Random bits should not overflow - TODO test")
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
            Message::Generate { msg_id } => {
                let response = Message::GenerateOk {
                    msg_id: self.get_next_msg_id(),
                    in_reply_to: msg_id,
                    id: self.generate_unique_id(),
                };
                ctx.reply(response_builder, response, self).await;
            }
            Message::GenerateOk { .. } => {}
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
    Generate {
        msg_id: u64,
    },
    GenerateOk {
        msg_id: u64,
        in_reply_to: u64,
        #[serde(with = "ulid_as_uuid")]
        id: ulid::Ulid,
    },
}
