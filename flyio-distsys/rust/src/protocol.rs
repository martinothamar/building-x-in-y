use std::{error::Error, pin::Pin};

use async_stream::stream;
use tokio::io::{self, AsyncBufReadExt, AsyncWriteExt, BufReader, BufWriter};
use tokio_stream::Stream;

use crate::{request::RequestEnvelope, response::ResponseEnvelope};

pub struct Protocol {
    incoming: BufReader<io::Stdin>,
    outgoing: BufWriter<io::Stdout>,
}

impl Protocol {
    const BUF_SIZE: usize = 1024 * 8;

    pub fn new() -> Self {
        let stdin = io::stdin();
        let stdout = io::stdout();
        Self {
            incoming: BufReader::with_capacity(Self::BUF_SIZE, stdin),
            outgoing: BufWriter::with_capacity(Self::BUF_SIZE, stdout),
        }
    }

    pub fn split(self) -> (ProtocolReader, ProtocolWriter) {
        (
            ProtocolReader {
                incoming: self.incoming,
                buffer: Vec::with_capacity(Self::BUF_SIZE),
            },
            ProtocolWriter {
                outgoing: self.outgoing,
                buffer: Vec::with_capacity(Self::BUF_SIZE),
                next_msg_id: 0,
            },
        )
    }
}

pub struct ProtocolReader {
    incoming: BufReader<io::Stdin>,
    buffer: Vec<u8>,
}

impl ProtocolReader {
    pub fn recv_stream(mut self) -> Pin<Box<impl Stream<Item = Result<RequestEnvelope, std::io::Error>>>> {
        Box::pin(stream! {
            loop {
                let result = self.incoming.read_until(b'\n', &mut self.buffer).await;
                match result {
                    Ok(read) => {
                        let msg: RequestEnvelope = serde_json::from_slice(&self.buffer[..read])?;
                        yield Ok(msg);
                    }
                    Err(e) => yield Err(e),
                }

                self.buffer.clear();
            }
        })
    }
}

pub struct ProtocolWriter {
    outgoing: BufWriter<io::Stdout>,
    buffer: Vec<u8>,
    next_msg_id: usize,
}

impl ProtocolWriter {
    pub async fn send(&mut self, response: ResponseEnvelope) -> Result<(), Box<dyn Error>> {
        self.buffer.clear();
        serde_json::to_writer(&mut self.buffer, &response)?;
        self.buffer.push(b'\n');
        self.outgoing.write_all(&self.buffer).await?;
        self.outgoing.flush().await?;
        Ok(())
    }

    pub fn get_next_msg_id(&mut self) -> usize {
        let id = self.next_msg_id;
        self.next_msg_id += 1;
        id
    }
}
