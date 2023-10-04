use serde::Deserialize;

use crate::response::{Response, ResponseEnvelope};

#[derive(Deserialize, Debug, Clone)]
pub struct RequestEnvelope {
    pub src: String,
    pub dest: String,
    pub body: Request,
}

impl RequestEnvelope {
    #[allow(clippy::wrong_self_convention)]
    pub fn to_response(self, body: Response) -> ResponseEnvelope {
        ResponseEnvelope {
            src: self.dest,
            dest: self.src,
            body,
        }
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
}
