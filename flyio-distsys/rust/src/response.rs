use serde::Serialize;

#[derive(Serialize, Debug, Clone)]
pub struct ResponseEnvelope {
    pub src: String,
    pub dest: String,
    pub body: Response,
}

#[derive(Serialize, Debug, Clone)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum Response {
    InitOk {
        msg_id: usize,
        in_reply_to: usize,
    },
    EchoOk {
        msg_id: usize,
        in_reply_to: usize,
        echo: String,
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
