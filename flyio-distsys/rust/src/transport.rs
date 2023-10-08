use std::{error::Error, pin::Pin};

use async_stream::stream;
use tokio::io::{self, AsyncBufReadExt, AsyncWriteExt, BufReader, BufWriter};
use tokio_stream::Stream;

use crate::{request::RequestEnvelope, response::ResponseEnvelope};

pub struct Transport {
    incoming: BufReader<io::Stdin>,
    outgoing: BufWriter<io::Stdout>,
}

impl Transport {
    const BUF_SIZE: usize = 1024 * 8;

    pub fn new() -> Self {
        let stdin = io::stdin();
        let stdout = io::stdout();
        Self {
            incoming: BufReader::with_capacity(Self::BUF_SIZE, stdin),
            outgoing: BufWriter::with_capacity(Self::BUF_SIZE, stdout),
        }
    }

    pub fn split(self) -> (TransportReader, TransportWriter) {
        (
            TransportReader {
                incoming: self.incoming,
                buffer: Vec::with_capacity(Self::BUF_SIZE),
            },
            TransportWriter {
                outgoing: self.outgoing,
                buffer: Vec::with_capacity(Self::BUF_SIZE),
            },
        )
    }
}

pub struct TransportReader {
    incoming: BufReader<io::Stdin>,
    buffer: Vec<u8>,
}

impl TransportReader {
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

pub struct TransportWriter {
    outgoing: BufWriter<io::Stdout>,
    buffer: Vec<u8>,
}

impl TransportWriter {
    pub async fn send(&mut self, responses: &[ResponseEnvelope]) -> Result<(), Box<dyn Error>> {
        self.buffer.clear();

        for response in responses {
            serde_json::to_writer(&mut self.buffer, response)?;
            self.buffer.push(b'\n');
        }
        self.outgoing.write_all(&self.buffer).await?;
        self.outgoing.flush().await?;
        Ok(())
    }
}
