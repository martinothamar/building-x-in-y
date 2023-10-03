use std::fmt::Display;

use serde::Deserialize;

use crate::response::{Response, ResponseEnvelope};

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct NodeId(Option<String>);

impl NodeId {
    pub fn new() -> Self {
        Self(None)
    }

    pub fn init(&mut self, id: &str) {
        self.0 = Some(id.to_string());
    }
}

impl Display for NodeId {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match &self.0 {
            Some(s) => f.write_str(s),
            None => f.write_str("N/A"),
        }
    }
}

#[derive(Deserialize, Debug, Clone)]
pub struct RequestEnvelope {
    pub src: String,
    pub dest: String,
    pub body: Request,
}

impl RequestEnvelope {
    pub fn to_response(&self, body: Response) -> ResponseEnvelope {
        ResponseEnvelope {
            src: self.dest.clone(),
            dest: self.src.clone(),
            body,
        }
    }
}

#[derive(Deserialize, Debug, Clone)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum Request {
    Init {
        msg_id: usize,
        node_id: String,
        node_ids: Vec<String>,
    },
    Read {
        msg_id: usize,
        key: usize,
    },
    Echo {
        msg_id: usize,
        echo: String,
    },
}
