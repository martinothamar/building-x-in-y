use serde::Serialize;
use ulid::serde::ulid_as_uuid;

use crate::request::RequestMetadata;

pub struct ResponseBuilder {
    pub request: RequestMetadata,
}

impl ResponseBuilder {
    pub fn new(request: RequestMetadata) -> Self {
        Self { request }
    }

    pub fn build(self, body: Response) -> ResponseEnvelope {
        ResponseEnvelope {
            src: self.request.dest,
            dest: self.request.src,
            body,
        }
    }
}

#[derive(Serialize, Debug, Clone)]
pub struct ResponseEnvelope {
    pub src: String,
    pub dest: String,
    pub body: Response,
}

#[derive(Serialize, Debug, Clone)]
#[serde(tag = "type", rename_all = "snake_case")]
#[allow(clippy::enum_variant_names)]
pub enum Response {
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
    Broadcast {
        msg_id: u64,
        message: i64,
    },
    ReadOk {
        msg_id: u64,
        in_reply_to: u64,
        messages: Vec<i64>,
    },
}
