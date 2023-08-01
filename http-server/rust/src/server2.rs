use std::cell::RefCell;
use std::io;
use std::net::SocketAddr;
use std::net::TcpListener;
use std::os::fd::AsRawFd;
use std::os::fd::RawFd;
use std::rc::Rc;
use std::sync::Arc;
use std::thread;
use std::time::Duration;

use anyhow::anyhow;
use anyhow::Context;
use anyhow::Result;
use io_uring::cqueue;
use io_uring::opcode;
use io_uring::squeue;
use io_uring::squeue::PushError;
use io_uring::types;
use io_uring::IoUring;
use slab::Slab;
use socket2::Domain;
use socket2::Socket;
use socket2::Type;
use tokio::io::unix::AsyncFd;
use tokio::io::Interest;
use tokio::runtime;
use tracing::error;
use tracing::info;

use crate::buf_ring;
use crate::buf_ring::FixedSizeBufRing;
use crate::linux;
use crate::resp;

struct ThreadWorker {
    _id: u64,
    processor: u16,
    name: String,
}

#[macro_export]
macro_rules! log_info {
    ($w:ident, $m:literal) => (info!(concat!("[{}] ", $m), $w.borrow().name));
    ($w:ident, $m:literal, $($arg:expr),+) => (info!(concat!("[{}] ", $m), $w.borrow().name, $($arg),+));
}

#[macro_export]
macro_rules! log_error {
    ($w:ident, $m:literal) => (error!(concat!("[{}] ", $m), $w.borrow().name));
    ($w:ident, $m:literal, $($arg:expr),+) => (error!(concat!("[{}] ", $m), $w.borrow().name, $($arg),+));
}

impl ThreadWorker {
    fn new(id: u64, processor: u16, name: String) -> Self {
        Self {
            _id: id,
            processor,
            name,
        }
    }
}

pub fn start() -> Result<()> {
    let (core_count, processors_to_use) = linux::thread_per_core();

    let mut threads = Vec::with_capacity(core_count as usize);

    for ti in 0..core_count {
        let processors_to_use = Arc::clone(&processors_to_use);

        let t = thread::Builder::new()
            .name(format!("httpsrv-worker-{ti}"))
            .spawn(move || {
                let thread = thread::current();
                let name = thread.name().unwrap().to_owned();
                let id = unsafe { libc::pthread_self() };
                let processor = processors_to_use[ti as usize];
                let worker = Rc::new(RefCell::new(ThreadWorker::new(id, processor as u16, name)));

                log_info!(worker, "thread starting");

                linux::pin_thread(processor);
                log_info!(worker, "thread pinned to processor: {}", processor);
                run_thread(worker)
            })?;

        threads.push(t);
    }

    for t in threads {
        t.join().map_err(|e| anyhow!("Thread panicked: {:?}", e))??;
    }

    Ok(())
}

fn create_tcp_listener() -> Result<TcpListener> {
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

    Ok(socket.into())
}

#[derive(Clone, Debug, PartialEq, Eq, PartialOrd, Ord, Default)]
enum Operation {
    #[default]
    None,
    Accept,
    Read(i32),
    Write(i32),
}

fn register_buffer_rings(worker: &ThreadWorker, ring: &mut IoUring) -> Result<FixedSizeBufRing> {
    const RING_ENTRIES: u16 = 128;
    const PAGE_SIZE: usize = 4096;
    let bg_id = worker.processor;

    let buf_ring = buf_ring::Builder::new(bg_id)
        .ring_entries(RING_ENTRIES)
        .buf_len(PAGE_SIZE)
        .build()?;
    buf_ring.rc.register(ring)?;

    Ok(buf_ring)
}

fn run_thread(worker: Rc<RefCell<ThreadWorker>>) -> Result<()> {
    let rt = runtime::Builder::new_current_thread()
        .enable_time()
        .enable_io()
        .build()
        .context("failed to create async executor")?;
    let local = tokio::task::LocalSet::new();

    let ring = IoUring::builder()
        .setup_coop_taskrun()
        .setup_cqsize(256)
        // .setup_sqpoll(100)
        // .setup_sqpoll_cpu((worker.processor + 1) as u32)
        .build(128)
        .context("failed to initialize IO uring")?;

    local.spawn_local(event_loop(worker, ring));

    rt.block_on(local);

    #[allow(unreachable_code)]
    Ok(())
}

#[derive(Clone)]
struct RingHandle {
    inner: Rc<RefCell<IoUring>>,
}

impl RingHandle {
    #[inline]
    fn register_files(&self, fds: &[RawFd]) -> io::Result<()> {
        self.inner.borrow().submitter().register_files(fds)
    }

    #[inline]
    fn push_sq(&self, entry: &squeue::Entry) -> Result<(), PushError> {
        unsafe { self.inner.borrow_mut().submission().push(entry) }
    }

    #[inline]
    fn sync_sq(&self) {
        self.inner.borrow_mut().submission().sync();
    }

    #[inline]
    fn sync_cq(&self) {
        self.inner.borrow_mut().completion().sync();
    }

    #[inline]
    fn submit(&self) -> io::Result<usize> {
        self.inner.borrow().submitter().submit()
    }
}

impl Iterator for &RingHandle {
    type Item = cqueue::Entry;

    fn next(&mut self) -> Option<Self::Item> {
        self.inner.borrow_mut().completion().next()
    }
}

impl AsRawFd for RingHandle {
    fn as_raw_fd(&self) -> std::os::fd::RawFd {
        self.inner.borrow().as_raw_fd()
    }
}

async fn event_loop(worker: Rc<RefCell<ThreadWorker>>, mut ring: IoUring) -> Result<()> {
    let buf_ring = register_buffer_rings(&worker.borrow(), &mut ring)?;
    let bg_id = worker.borrow().processor;

    let mut operations: Slab<Operation> = Slab::with_capacity(64);

    let listener = create_tcp_listener().context("failed to create TCP listener")?;
    let listener_fd = types::Fd(listener.as_raw_fd());

    let ring = RingHandle {
        inner: Rc::new(RefCell::new(ring)),
    };

    let ring_fd = AsyncFd::with_interest(ring.clone(), Interest::READABLE)?;

    ring.register_files(&[listener.as_raw_fd()])?;

    {
        let accept_op = opcode::AcceptMulti::new(listener_fd);
        let listener_entry = accept_op.build().user_data(operations.insert(Operation::Accept) as _);

        ring.push_sq(&listener_entry)
            .context("failed to enqueue socket accept operation to IO uring")?;
        ring.sync_sq();
        ring.submit()?;
    }

    let active_connections = Rc::new(RefCell::new(0usize));
    let inc_active_connections = || *active_connections.borrow_mut() += 1;
    let dec_active_connections = || *active_connections.borrow_mut() -= 1;

    {
        let worker = Rc::clone(&worker);
        let active_connections = Rc::clone(&active_connections);
        tokio::task::spawn_local(async move {
            loop {
                tokio::time::sleep(Duration::from_secs(5)).await;
                log_info!(worker, "active connections: {}", *active_connections.borrow());
            }
        });
    };

    loop {
        let mut guard = ring_fd.readable().await?;

        // log_info!(worker, "reading cqe's");

        let ring = guard.get_inner();
        ring.sync_cq();
        for cqe in ring {
            let ret = cqe.result();

            let op_index = cqe.user_data() as usize;
            let op = &operations[op_index];

            log_info!(worker, "cqe received: op={:?}, ret={}", op, ret);

            if ret < 0 {
                let e = io::Error::from_raw_os_error(-ret);
                log_error!(worker, "error cqe: {}, {:?}", e, op);
                return Err(e.into());
            }

            match op {
                Operation::None => unreachable!(),
                Operation::Accept => {
                    // log_info!(worker, "accepted request");
                    let fd = ret;
                    let read_op = opcode::RecvMulti::new(types::Fd(fd), bg_id)
                        .build()
                        .user_data(operations.insert(Operation::Read(fd)) as _);
                    ring.push_sq(&read_op)?;
                    inc_active_connections();
                }
                Operation::Read(fd) => {
                    if ret == 0 {
                        // EOF
                        // log_info!(worker, "EOF for request");
                        operations.remove(op_index);
                        dec_active_connections();
                    } else {
                        let len = ret as usize;
                        let result = ret as usize;
                        assert_eq!(result, len);
                        let flags = cqe.flags();
                        let buf = buf_ring.rc.get_buf(buf_ring.clone(), result as u32, flags)?;
                        let buf = buf.as_slice();

                        let _str = std::str::from_utf8(buf)?;

                        if len >= 4 && buf[len - 4..len].eq(b"\r\n\r\n") {
                            let write_op = opcode::SendZc::new(
                                types::Fd(*fd),
                                resp::RESPONSE_HELLO_WORLD.as_ptr(),
                                resp::RESPONSE_HELLO_WORLD.len() as u32,
                            )
                            .build()
                            .user_data(operations.insert(Operation::Write(*fd)) as _);
                            ring.push_sq(&write_op)?;
                        } else {
                            log_error!(worker, "got response, but couldn't reach end of request");
                        }
                    }
                }
                Operation::Write(_fd) => {
                    // log_info!(worker, "wrote {}", len);
                }
            }
        }

        ring.sync_sq();
        ring.submit()?;

        guard.clear_ready();
    }

    #[allow(unreachable_code)]
    Ok(())
}
