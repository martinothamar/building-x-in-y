use std::{error::Error, pin::Pin};

use async_stream::stream;
use tokio::io::{self, AsyncBufReadExt, AsyncWriteExt, BufReader, BufWriter};
use tokio_stream::Stream;

use crate::message::MessageEnvelope;

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

impl Default for Transport {
    fn default() -> Self {
        Self::new()
    }
}

pub struct TransportReader {
    incoming: BufReader<io::Stdin>,
    buffer: Vec<u8>,
}

impl TransportReader {
    pub fn recv_stream<T: serde::de::DeserializeOwned>(
        mut self,
    ) -> Pin<Box<impl Stream<Item = Result<MessageEnvelope<T>, std::io::Error>>>> {
        Box::pin(stream! {
            loop {
                let result = self.incoming.read_until(b'\n', &mut self.buffer).await;
                match result {
                    Ok(read) => {
                        let msg: MessageEnvelope<T> = serde_json::from_slice(&self.buffer[..read])?;
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
    pub async fn send<T: serde::Serialize>(&mut self, messages: &[MessageEnvelope<T>]) -> Result<(), Box<dyn Error>> {
        self.buffer.clear();

        for message in messages {
            serde_json::to_writer(&mut self.buffer, message)?;
            self.buffer.push(b'\n');
        }
        self.outgoing.write_all(&self.buffer).await?;
        self.outgoing.flush().await?;
        Ok(())
    }
}
