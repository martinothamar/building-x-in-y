use std::cmp::Ordering;
use std::collections::HashMap;
use std::future::Future;
use std::net;
use std::pin::Pin;
use std::sync::Arc;
use std::task::{Poll, Waker, Context};
use std::{any::Any, cell::RefCell, fmt::Display, io, iter, net::SocketAddr, rc::Rc, thread};

use socket2::{Domain, Socket, Type};
use thiserror::Error;
use tokio::signal::unix::{signal, Signal, SignalKind};
use tokio::task::JoinError;
use tokio_uring::{
    buf::{fixed::FixedBufRegistry, IoBufMut},
    net::{TcpListener, TcpStream},
    Runtime,
};
use tracing::*;

mod linux;

const RESPONSE: &'static [u8] = b"HTTP/1.1 200 OK\nContent-Type: text/plain\nContent-Length: 13\n\nHello, world!";
const SERVICE_UNAVAILABLE: &'static [u8] = b"HTTP/1.1 503 Service Unavailable\n\n";

const MAX_CONNECTIONS_PER_WORKER: usize = 1024 * 4;
const CONNECTION_BUFFER_SIZE: usize = 1024 * 2;

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
                    state: Rc::new(RefCell::new(WorkerState::new())),
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
    state: Rc<RefCell<WorkerState>>,
    active_connection_count: Rc<RefCell<usize>>,
}

#[derive(PartialEq, Eq, Clone, Copy, PartialOrd, Ord)]
#[allow(dead_code)]
enum WorkerStateValue {
    Starting,
    Running,
    ShuttingDown,
    ShutDown,
}

struct WorkerState {
    current: WorkerStateValue,
}

impl WorkerState {
    fn new() -> Self{
        Self { current: WorkerStateValue::Starting }
    }

    fn transition_to(&mut self, state: WorkerStateValue) {
        if state < self.current {
            unreachable!("Should never try tro transition backwards");
        }
        self.current = state;
    }
}

struct WaitForShutdown {
    state: Rc<RefCell<WorkerState>>,
    waker: Option<Waker>,
}

impl Future for WaitForShutdown {
    type Output = ();

    fn poll(mut self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Self::Output> {
        let ordering = {
            let state = self.state.borrow();
            state.current.cmp(&WorkerStateValue::ShuttingDown)
        };
        info!("Polling WaitForShutdown");
        match ordering {
            Ordering::Less => {
                self.waker = Some(cx.waker().clone());
                Poll::Pending
            },
            Ordering::Equal => Poll::Ready(()),
            Ordering::Greater => Poll::Ready(()),
        }
    }
}

impl Display for ThreadWorker {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", &self.name)
    }
}

impl ThreadWorker {
    fn signal_shutdown(&self) {
        let mut flag = self.state.borrow_mut();
        if flag.current < WorkerStateValue::ShuttingDown {
            flag.transition_to(WorkerStateValue::ShuttingDown);
        }
    }

    fn is_shutting_down(&self) -> bool {
        self.get_state() == WorkerStateValue::ShuttingDown
    }

    fn get_state(&self) -> WorkerStateValue {
        self.state.borrow().current
    }

    async fn wait_for_shutdown(&self) {
        let value = WaitForShutdown {
            state: Rc::clone(&self.state),
            waker: None,
        };
        value.await;
    }

    fn increment_active_connections(&self) -> usize {
        // TODO - UnsafeCell instead?
        let mut count = self.active_connection_count.borrow_mut();
        let current = *count;
        *count += 1;
        current
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
    const SIGTERM: SignalKind = SignalKind::terminate();
    const SIGINT: SignalKind = SignalKind::interrupt();

    fn attach(s: SignalKind, name: &str) -> io::Result<Signal> {
        signal(s).map_err(|e| {
            error!("[{}] couldn't attach listener for OS signal: {:?}", name, s);
            e
        })
    }

    let mut sigterm = attach(SIGTERM, &worker.name)?;
    let mut sigint = attach(SIGINT, &worker.name)?;

    let s = tokio::select! {
        _ = sigterm.recv() => SIGTERM,
        _ = sigint.recv() => SIGINT,
    };

    info!("[{}] received OS signal: {:?}, shutting down..", &worker.name, s);
    worker.signal_shutdown();
    Ok(())
}

async fn listen_for_clients(worker: Rc<ThreadWorker>, listener: TcpListener, registry: FixedBufRegistry<Vec<u8>>) {
    while !worker.is_shutting_down() {

        info!("listening...");
        let client_result = tokio::select! {
            _ = worker.wait_for_shutdown() => None,
            client_result = listener.accept() => Some(client_result),
        };
        info!("got event...");

        let Some(client_result) = client_result else {
            info!("[{}] exiting listener loop", &worker.name);
            break;
        };

        match client_result {
            Err(e) => error!("[{}] failed to accept client: {}", &worker.name, e),
            Ok((stream, addr)) => {
                let current = worker.increment_active_connections();
                if current == MAX_CONNECTIONS_PER_WORKER - 1 {
                    let (res, _) = stream.write_all(SERVICE_UNAVAILABLE).await;
                    debug!("[{}] request denied, backpressure applied", &worker.name);
                    worker.decrement_active_connections();
                    let _ = res.unwrap_or_else(|e| {
                        error!("[{}] failed to apply backpressure to client: {}", &worker.name, e);
                        ()
                    });
                    let _ = stream.shutdown(std::net::Shutdown::Write);
                } else {
                    let worker = Rc::clone(&worker);
                    let registry = registry.clone();
                    tokio_uring::spawn(async move {
                        handle_client(Rc::clone(&worker), current, stream, addr, registry).await;
                        worker.decrement_active_connections();
                    });
                }
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
            let connection_buffers = iter::repeat(vec![0; CONNECTION_BUFFER_SIZE]).take(MAX_CONNECTIONS_PER_WORKER);
            let registry = FixedBufRegistry::new(connection_buffers);
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
    connection_index: usize,
    client_stream: TcpStream,
    _client_addr: SocketAddr,
    registry: FixedBufRegistry<T>,
) {
    let mut buffer = registry
        .check_out(connection_index)
        .expect("Should be available since we keep track of active connections");

    let mut n = 0;
    loop {
        let (result, read_buffer) = client_stream.read_fixed(buffer).await;
        buffer = {
            let read = result.unwrap_or_else(|e| {
                warn!("[{}] error reading response: {}", &worker.name, e);
                0
            });
            if read == 0 {
                break;
            }

            n += read;

            if read >= 4 && read_buffer[n - 4..n].eq(b"\r\n\r\n") {
                let (res, _) = client_stream.write_all(RESPONSE).await;
                debug!(
                    "[{}] request received:\n{}",
                    &worker.name,
                    std::str::from_utf8(&read_buffer[0..n]).unwrap_or("error reading body")
                );
                res.unwrap_or_else(|e| {
                    error!("[{}] error writing response: {}", &worker.name, e);
                });
                n = 0;
            }

            read_buffer
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
