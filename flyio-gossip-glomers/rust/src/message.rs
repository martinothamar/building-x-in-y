use std::collections::HashMap;

use serde::{Deserialize, Serialize};
use ulid::serde::ulid_as_uuid;

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct MessageEnvelope {
    pub src: String,
    pub dest: String,
    pub body: Message,
}

pub struct MessageMetadata {
    pub src: String,
    pub dest: String,
}

impl MessageEnvelope {
    pub fn split(self) -> (MessageMetadata, Message) {
        (
            MessageMetadata {
                src: self.src,
                dest: self.dest,
            },
            self.body,
        )
    }
}

pub struct MessageReplyBuilder {
    pub request: MessageMetadata,
}

impl MessageReplyBuilder {
    pub fn new(request: MessageMetadata) -> Self {
        Self { request }
    }

    pub fn build(self, body: Message) -> MessageEnvelope {
        MessageEnvelope {
            src: self.request.dest,
            dest: self.request.src,
            body,
        }
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
    Echo {
        msg_id: u64,
        echo: String,
    },
    Generate {
        msg_id: u64,
    },
    Topology {
        msg_id: u64,
        topology: HashMap<String, Vec<String>>,
    },
    Broadcast {
        msg_id: u64,
        message: u64,
    },
    Read {
        msg_id: u64,
    },

    InitOk {
        msg_id: u64,
        in_reply_to: u64,
    },
    EchoOk {
        msg_id: u64,
        in_reply_to: u64,
        echo: String,
    },
    GenerateOk {
        msg_id: u64,
        in_reply_to: u64,
        #[serde(with = "ulid_as_uuid")]
        id: ulid::Ulid,
    },
    TopologyOk {
        msg_id: u64,
        in_reply_to: u64,
    },
    BroadcastOk {
        msg_id: u64,
        in_reply_to: u64,
    },
    ReadOk {
        msg_id: u64,
        in_reply_to: u64,
        messages: Vec<u64>,
    },
}
