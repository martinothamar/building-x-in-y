use serde::{Deserialize, Serialize};

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct MessageEnvelope<T> {
    pub src: String,
    pub dest: String,
    pub body: T,
}

pub struct MessageMetadata {
    pub src: String,
    pub dest: String,
}

impl<T> MessageEnvelope<T> {
    pub fn split(self) -> (MessageMetadata, T) {
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

    pub fn build<T>(self, body: T) -> MessageEnvelope<T> {
        MessageEnvelope::<T> {
            src: self.request.dest,
            dest: self.request.src,
            body,
        }
    }
}
