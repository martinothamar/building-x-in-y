use serde::Serialize;
use ulid::serde::ulid_as_uuid;

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
    // ReadOk {
    //     msg_id: usize,
    //     in_reply_to: usize,
    //     value: usize,
    // },
    // Error {
    //     in_reply_to: usize,
    //     code: usize,
    //     text: String,
    // },
}
