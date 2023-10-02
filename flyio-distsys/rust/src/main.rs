use std::{error::Error, pin::Pin};

use async_stream::stream;
use serde::{Deserialize, Serialize};
use tokio::{
    io::{self, AsyncBufReadExt, AsyncWriteExt, BufReader, BufWriter},
    runtime,
};
use tokio_stream::{Stream, StreamExt};

fn main() -> Result<(), Box<dyn Error>> {
    let rt = runtime::Builder::new_current_thread().enable_all().build()?;

    rt.block_on(async {
        let mut logger = Logger::new();

        logger.log("Starting!\n").await;

        let proto = Protocol::new();

        let mut stream = proto.stream();
        while let Some(result) = stream.next().await {
            match result {
                Ok(msg) => {
                    // Handle message
                    logger.log(format!("Got message: {:?}\n", &msg)).await;

                    let response = match msg.body {
                        Request::Init {
                            msg_id,
                            node_id,
                            node_ids,
                        } => Some(Response::InitOk { in_reply_to: msg_id }),
                        Request::Read { msg_id, key } => None,
                    };
                }
                Err(e) => logger.log(format!("Error: {}\n", e)).await,
            }
        }

        logger.log("Exiting...\n").await;
    });

    Ok(())
}

struct Protocol {
    inner: BufReader<io::Stdin>,
    buffer: Vec<u8>,
}

impl Protocol {
    fn new() -> Self {
        let stdin = io::stdin();
        Self {
            inner: BufReader::with_capacity(1024 * 8, stdin),
            buffer: Vec::with_capacity(1024 * 8),
        }
    }

    fn stream(mut self) -> Pin<Box<impl Stream<Item = Result<RequestEnvelope, std::io::Error>>>> {
        Box::pin(stream! {
            loop {
                let result = self.inner.read_until(b'\n', &mut self.buffer).await;
                match result {
                    Ok(read) => {
                        // {"id":4,"src":"c4","dest":"n1","body":{"type":"init","node_id":"n1","node_ids":["n1","n2","n3","n4","n5"],"msg_id":1}}
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

#[derive(Deserialize, Debug, Clone)]
struct RequestEnvelope {
    src: String,
    dest: String,
    body: Request,
}

impl RequestEnvelope {
    fn to_response(self, body: Response) -> ResponseEnvelope {
        ResponseEnvelope {
            src: self.dest,
            dest: self.src,
            body,
        }
    }
}

#[derive(Deserialize, Debug, Clone)]
#[serde(tag = "type", rename_all = "snake_case")]
enum Request {
    Init {
        msg_id: usize,
        node_id: String,
        node_ids: Vec<String>,
    },
    Read {
        msg_id: usize,
        key: usize,
    },
}

#[derive(Serialize, Debug, Clone)]
struct ResponseEnvelope {
    src: String,
    dest: String,
    body: Response,
}

#[derive(Serialize, Debug, Clone)]
#[serde(tag = "type", rename_all = "snake_case")]
enum Response {
    InitOk {
        in_reply_to: usize,
    },
    ReadOk {
        msg_id: usize,
        in_reply_to: usize,
        value: usize,
    },
    Error {
        in_reply_to: usize,
        code: usize,
        text: String,
    },
}

struct Logger {
    inner: BufWriter<io::Stderr>,
}

impl Logger {
    fn new() -> Self {
        let stderr: io::Stderr = io::stderr();
        Self {
            inner: BufWriter::new(stderr),
        }
    }

    async fn log<T: AsRef<[u8]>>(&mut self, data: T) {
        let buf = data.as_ref();
        self.inner
            .write_all(buf)
            .await
            .unwrap_or_else(|_| std::process::exit(420));
        self.inner.flush().await.unwrap_or_else(|_| std::process::exit(420));
    }
}
