use std::thread::{self, JoinHandle};
use tokio::{
    io::{self, AsyncWriteExt, BufWriter},
    runtime,
    sync::mpsc::{self, UnboundedSender},
};

pub struct Logger {
    producer: UnboundedSender<Vec<u8>>,
}

impl Logger {
    pub fn new() -> (Self, JoinHandle<()>) {
        let (producer, mut consumer) = mpsc::unbounded_channel::<Vec<u8>>();

        let join_handle = thread::spawn(move || {
            let runtime = runtime::Builder::new_current_thread().enable_all().build().unwrap();

            runtime.block_on(async move {
                let stderr: io::Stderr = io::stderr();
                let mut writer = BufWriter::with_capacity(1024 * 8, stderr);
                while let Some(data) = consumer.recv().await {
                    let buf = data.as_ref();
                    writer.write_all(buf).await.unwrap_or_else(|_| std::process::exit(420));
                    writer.flush().await.unwrap_or_else(|_| std::process::exit(420));
                }
            });
        });

        let logger = Self { producer };

        (logger, join_handle)
    }

    pub fn log<S: Into<String>>(&mut self, data: S) {
        let s: String = data.into();
        self.producer.send(s.into()).expect("channel should not be closed");
    }
}
