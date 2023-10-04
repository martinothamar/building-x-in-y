use tokio::io::{self, AsyncWriteExt, BufWriter};

pub struct Logger {
    inner: BufWriter<io::Stderr>,
}

impl Logger {
    pub fn new() -> Self {
        let stderr: io::Stderr = io::stderr();
        Self {
            inner: BufWriter::new(stderr),
        }
    }

    pub async fn log<T: AsRef<[u8]>>(&mut self, data: T) {
        let buf = data.as_ref();
        self.inner
            .write_all(buf)
            .await
            .unwrap_or_else(|_| std::process::exit(420));
        self.inner.flush().await.unwrap_or_else(|_| std::process::exit(420));
    }
}
