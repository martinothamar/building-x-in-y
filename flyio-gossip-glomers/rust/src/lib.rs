#![feature(hash_raw_entry)]

use async_trait::async_trait;
use std::{error::Error, time::Duration};

use logger::Logger;
use message::{MessageEnvelope, MessageReplyBuilder};
use mimalloc::MiMalloc;
use node::{NodeId, Topology};
use tokio::{runtime, time};
use tokio_stream::StreamExt;
use transport::{Transport, TransportWriter};

#[global_allocator]
static GLOBAL: MiMalloc = MiMalloc;

pub mod logger;
pub mod message;
pub mod node;
pub mod transport;

pub struct Runtime {
    inner: runtime::Runtime,
    gossip_tick_interval: Duration,
}

impl Runtime {
    pub fn new() -> Result<Self, Box<dyn Error>> {
        Self::new_with_gossip_tick_interval(Duration::from_millis(500))
    }

    pub fn new_with_gossip_tick_interval(gossip_tick_interval: Duration) -> Result<Self, Box<dyn Error>> {
        let runtime = runtime::Builder::new_current_thread().enable_all().build()?;
        Ok(Self {
            inner: runtime,
            gossip_tick_interval,
        })
    }

    pub fn run<TNode: MaelstromNode<TMessage>, TMessage: serde::Serialize + serde::de::DeserializeOwned>(
        self,
        mut node: TNode,
    ) {
        self.inner.block_on(async {
            let (mut logger, logger_thread) = Logger::new();

            let topology = Topology::new();
            logger.log(format!("[{}] Starting!\n", node.id()));

            let (receiver, sender) = Transport::new().split();
            let mut ctx: RuntimeContext = RuntimeContext {
                topology,
                logger,
                sender,
            };

            let mut stream = receiver.recv_stream();
            let mut gossip_timer = time::interval(self.gossip_tick_interval);

            loop {
                tokio::select! {
                    _ = gossip_timer.tick() => {
                        node.handle_gossip_tick(&mut ctx).await;
                    }
                    result = stream.next() => {
                        let Some(result) = result else {
                            ctx.logger.log(format!("[{}] End of messages\n", node.id()));
                            break;
                        };
                        match result {
                            Ok(msg) => node.handle_message(msg, &mut ctx).await,
                            Err(e) => ctx.logger.log(format!("[{}] Error: {}\n", node.id(), e)),
                        };
                    }
                }
            }

            logger_thread.join().expect("Logger thread should not panic");

            ctx.logger.log(format!("[{}] Exiting...\n", node.id()));
        });
    }
}

pub struct RuntimeContext {
    pub topology: Topology,
    pub logger: Logger,
    pub sender: TransportWriter,
}

impl RuntimeContext {
    #[inline]
    pub async fn reply<TNode: MaelstromNode<TMessage>, TMessage: serde::Serialize>(
        &mut self,
        builder: MessageReplyBuilder,
        body: TMessage,
        node: &TNode,
    ) {
        match self.sender.send(&[builder.build(body)]).await {
            Ok(_) => {}
            Err(e) => self.logger.log(format!("[{}] Error: {}\n", node.id(), e)),
        };
    }

    #[inline]
    pub async fn send<TNode: MaelstromNode<TMessage>, TMessage: serde::Serialize>(
        &mut self,
        bodies: &[MessageEnvelope<TMessage>],
        node: &TNode,
    ) {
        match self.sender.send(bodies).await {
            Ok(_) => {}
            Err(e) => self.logger.log(format!("[{}] Error: {}\n", node.id(), e)),
        };
    }
}

#[async_trait]
pub trait MaelstromNode<T> {
    fn init(&mut self, id: String);

    fn id(&self) -> &NodeId;

    fn id_str(&self) -> &str;

    async fn handle_message(&mut self, msg: MessageEnvelope<T>, ctx: &mut RuntimeContext);

    async fn handle_gossip_tick(&mut self, _ctx: &mut RuntimeContext);
}
