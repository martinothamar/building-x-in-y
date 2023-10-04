use std::collections::HashMap;

use serde::Deserialize;

#[derive(Deserialize, Debug, Clone)]
pub struct RequestEnvelope {
    pub src: String,
    pub dest: String,
    pub body: Request,
}

pub struct RequestMetadata {
    pub src: String,
    pub dest: String,
}

impl RequestEnvelope {
    pub fn split(self) -> (RequestMetadata, Request) {
        (
            RequestMetadata {
                src: self.src,
                dest: self.dest,
            },
            self.body,
        )
    }
}

#[derive(Deserialize, Debug, Clone)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum Request {
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
        message: i64,
    },
    Read {
        msg_id: u64,
    },
}
