use std::collections::HashMap;
use std::net;
use std::sync::Arc;
use std::{any::Any, cell::RefCell, fmt::Display, io, iter, net::SocketAddr, rc::Rc, thread};

use socket2::{Domain, Socket, Type};
use thiserror::Error;
use tokio::signal::unix::{signal, SignalKind};
use tokio::task::JoinError;
use tokio_uring::{
    buf::{
        fixed::{FixedBuf, FixedBufRegistry},
        IoBufMut,
    },
    net::{TcpListener, TcpStream},
    Runtime,
};
use tracing::*;

mod linux;

const RESPONSE: &'static [u8] = b"HTTP/1.1 200 OK\nContent-Type: text/plain\nContent-Length: 13\n\nHello, world!";

#[derive(Error, Debug)]
pub enum ServerError {
    #[error("failed to parse address to bind to: {0}")]
    AddressParseError(#[from] net::AddrParseError),
    #[error("failed to set socket opts: {0}")]
    SocketOpts(i32),
    #[error("failed on IO: {0}")]
    IOError(#[from] io::Error),
    #[error("failed to spawn thread: {0}")]
    ThreadSpawn(io::Error),
    #[error("thead panic: {0:?}")]
    ThreadPanic(Box<dyn Any + Send>),
    #[error("task panic: {0}")]
    TaskPanic(#[from] JoinError),
}

pub fn start() -> Result<(), ServerError> {
    let cpu_info = linux::get_cpu_info();
    let core_count = linux::get_physical_core_count() / 2;
    let mut processors_to_use = HashMap::<usize, usize>::with_capacity(core_count as usize);
    cpu_info
        .processors
        .iter()
        .for_each(|p| _ = processors_to_use.insert(p.core_id, p.processor));
    let processors_to_use = Arc::new(processors_to_use.values().cloned().collect::<Vec<_>>());

    let mut threads = Vec::with_capacity(core_count as usize);

    for ti in 0..core_count {
        let processors_to_use = Arc::clone(&processors_to_use);
        let t = thread::Builder::new()
            .name(format!("httpsrv-worker-{ti}"))
            .spawn(move || {
                let worker = ThreadWorker {
                    name: thread::current().name().unwrap().to_owned(),
                    is_shutting_down: Rc::new(RefCell::new(false)),
                    active_connection_count: Rc::new(RefCell::new(0usize)),
                };

                info!("[{}] thread starting", &worker.name);
                let processor = processors_to_use[ti as usize];
                linux::pin_thread(processor);
                info!("[{}] thread pinned to processor: {}", &worker.name, processor);

                run_thread(worker)
            })
            .map_err(|e| ServerError::ThreadSpawn(e))?;

        threads.push(t);
    }

    for t in threads {
        t.join().map_err(|e| ServerError::ThreadPanic(e))??;
    }

    Ok(())
}

struct ThreadWorker {
    name: String,
    is_shutting_down: Rc<RefCell<bool>>,
    active_connection_count: Rc<RefCell<usize>>,
}

impl Display for ThreadWorker {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", &self.name)
    }
}

impl ThreadWorker {
    fn signal_shutdown(&self) {
        let mut flag = self.is_shutting_down.borrow_mut();
        *flag = true;
    }

    fn is_shutting_down(&self) -> bool {
        *self.is_shutting_down.borrow()
    }

    fn increment_active_connections(&self) {
        // TODO - UnsafeCell instead?
        let mut count = self.active_connection_count.borrow_mut();
        *count += 1;
    }

    fn decrement_active_connections(&self) {
        let mut count = self.active_connection_count.borrow_mut();
        *count -= 1;
    }

    fn create_tcp_listener(&self) -> Result<TcpListener, ServerError> {
        let addr_str = "0.0.0.0:8080";

        let addr: SocketAddr = addr_str.parse()?;
        let socket = Socket::new(Domain::IPV4, Type::STREAM, None)?;
        socket.set_reuse_address(true)?;
        socket.set_reuse_port(true)?;
        socket.set_nonblocking(true)?;
        socket.set_nodelay(true)?;
        socket.set_quickack(true)?;
        socket.bind(&addr.into())?;
        socket.listen(8192)?;

        Ok(TcpListener::from_std(socket.into()))
    }
}

async fn recv_os_signal(worker: Rc<ThreadWorker>) -> Result<(), ServerError> {
    const SIGNAL: SignalKind = SignalKind::terminate();
    let mut sigterm: tokio::signal::unix::Signal = signal(SIGNAL).map_err(|e| {
        error!(
            "[{}] couldn't attach listener for OS signal: {:?}",
            &worker.name, SIGNAL
        );
        e
    })?;

    sigterm.recv().await;
    worker.signal_shutdown();
    Ok(())
}

async fn listen_for_clients(worker: Rc<ThreadWorker>, listener: TcpListener, registry: FixedBufRegistry<Vec<u8>>) {
    while !worker.is_shutting_down() {
        let client_result = listener.accept().await;

        match client_result {
            Err(e) => error!("[{}] failed to accept client: {}", &worker.name, e),
            Ok((stream, addr)) => {
                worker.increment_active_connections();
                let worker = Rc::clone(&worker);
                let registry = registry.clone();
                tokio_uring::spawn(async move {
                    handle_client(Rc::clone(&worker), stream, addr, registry).await;
                    worker.decrement_active_connections();
                });
            }
        };
    }
}

fn run_thread(worker: ThreadWorker) -> Result<(), ServerError> {
    let mut uring_builder = tokio_uring::uring_builder();
    uring_builder.setup_sqpoll(100);

    let mut builder = tokio_uring::builder();
    builder.uring_builder(&uring_builder);

    let runtime = Runtime::new(&builder).map_err(|e| {
        error!("[{}] failed to construct runtime: {}", &worker.name, &e);
        e
    })?;

    let worker = Rc::new(worker);

    let result = {
        let worker = Rc::clone(&worker);
        runtime.block_on(async move {
            info!("[{}] iouring system started", &worker.name);

            let os_signal_listener = tokio_uring::spawn(recv_os_signal(Rc::clone(&worker)));

            // TODO align the buffers
            let registry = FixedBufRegistry::new(iter::repeat(vec![0; 4096]).take(64));
            registry.register().map_err(|e| {
                error!("[{}] failed to register fixed buffers: {}", &worker.name, &e);
                e
            })?;

            let listener = worker.create_tcp_listener()?;
            info!("[{}] http-server listening", &worker.name);

            let listener_handle = tokio_uring::spawn(listen_for_clients(Rc::clone(&worker), listener, registry));

            os_signal_listener.await.map_err(|e| {
                error!("[{}] OS signal listener panicked: {}", &worker.name, e);
                e
            })??;

            listener_handle.await.map_err(|e| {
                error!("[{}] listener panicked: {}", &worker.name, e);
                e
            })?;

            Ok(())
        })
    };

    info!("[{}] stopping thread", &worker.name);
    result
}

async fn handle_client<T: IoBufMut>(
    worker: Rc<ThreadWorker>,
    client_stream: TcpStream,
    client_addr: SocketAddr,
    registry: FixedBufRegistry<T>,
) {
    let mut buf_n = 0;
    let mut buf: Option<FixedBuf>;
    loop {
        if buf_n == 64 {
            let _ = client_stream.shutdown(std::net::Shutdown::Write);
            info!("[{}] client dropped, no more buffers: {}", &worker.name, &client_addr);
        }

        buf_n += 1;
        buf = registry.check_out(buf_n - 1);

        if !buf.is_none() {
            break;
        }
    }

    let mut buf = buf.unwrap();
    let mut n = 0;
    loop {
        let (result, buf_read) = client_stream.read_fixed(buf).await;
        buf = {
            let read = result.unwrap();
            if read == 0 {
                break;
            }

            n += read;

            if read >= 4 && buf_read[n - 4..n].eq(b"\r\n\r\n") {
                let (res, _) = client_stream.write_all(RESPONSE).await;
                debug!("[{}] request received:\n{}", &worker.name, std::str::from_utf8(&buf_read[0..n]).unwrap());
                let _ = res.unwrap();
                n = 0;
            }

            buf_read
        };
    }
    let _ = client_stream.shutdown(std::net::Shutdown::Write);
}

// #[cfg(test)]
// mod tests {
//     use super::*;

//     // #[test]
//     // fn it_works() {
//     //     let result = add(2, 2);
//     //     assert_eq!(result, 4);
//     // }
// }
