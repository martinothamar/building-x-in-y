use std::any::Any;
use std::io;
use std::mem::size_of;
use std::mem::MaybeUninit;
use std::net::SocketAddr;
use std::net::TcpListener;
use std::os::fd::AsRawFd;
use std::os::fd::FromRawFd;
use std::os::fd::RawFd;
use std::sync::Arc;
use std::thread;
use std::thread::ThreadId;

use anyhow::anyhow;
use anyhow::Context;
use anyhow::Error;
use anyhow::Result;
use io_uring::cqueue;
use io_uring::opcode;
use io_uring::squeue;
use io_uring::types;
use io_uring::types::Fd;
use io_uring::IoUring;
use io_uring::Submitter;
use slab::Slab;
use socket2::Domain;
use socket2::Socket;
use socket2::Type;
use tokio::runtime;
use tracing::error;
use tracing::info;

use crate::buf_ring;
use crate::buf_ring::FixedSizeBufRing;
use crate::linux;
use crate::resp;

struct ThreadWorker {
    id: u64,
    processor: u16,
    name: String,
}

#[macro_export]
macro_rules! log_info {
    ($w:ident, $m:literal) => (info!(concat!("[{}] ", $m), $w.name));
    ($w:ident, $m:literal, $($arg:expr),+) => (info!(concat!("[{}] ", $m), $w.name, $($arg),+));
}

#[macro_export]
macro_rules! log_error {
    ($w:ident, $m:literal) => (error!(concat!("[{}] ", $m), $w.name));
    ($w:ident, $m:literal, $($arg:expr),+) => (error!(concat!("[{}] ", $m), $w.name, $($arg),+));
}

impl ThreadWorker {
    fn new(id: u64, processor: u16, name: String) -> Self {
        Self { id, processor, name }
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
                let worker = ThreadWorker::new(id, processor as u16, name);

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

fn run_thread(worker: ThreadWorker) -> Result<()> {
    // let rt = runtime::Builder::new_current_thread()
    //     .build()
    //     .context("failed to create async executor")?;
    // let local = tokio::task::LocalSet::new();
    // let _guard = rt.enter();

    let mut ring = IoUring::builder()
        .setup_coop_taskrun()
        .setup_cqsize(256)
        .build(128)
        .context("failed to initialize IO uring")?;

    let listener = create_tcp_listener().context("failed to create TCP listener")?;
    let listener_fd = types::Fd(listener.as_raw_fd());

    ring.submitter().register_files(&[listener.as_raw_fd()])?;

    let accept_op = opcode::AcceptMulti::new(listener_fd);

    let mut operations: Slab<Operation> = Slab::with_capacity(64);

    let buf_ring = register_buffer_rings(&worker, &mut ring)?;
    let bg_id = worker.processor;

    let (submitter, mut sq, mut cq) = ring.split();

    let listener_entry = accept_op.build().user_data(operations.insert(Operation::Accept) as _);
    unsafe {
        sq.push(&listener_entry)
            .context("failed to enqueue socket accept operation to IO uring")?;
        sq.sync();
    }

    submitter
        .submit_and_wait(1)
        .context("failed to submit enqueued operations to IO uring")?;

    loop {
        if cq.is_empty() {
            submitter.submit_and_wait(1).context("failed to wait for cqe's")?;
            cq.sync();
        }

        for cqe in &mut cq {
            let ret = cqe.result();

            let op_index = cqe.user_data() as usize;
            let op = &operations[op_index];

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
                    unsafe { sq.push(&read_op)? };
                    sq.sync();
                }
                Operation::Read(fd) => {
                    if ret == 0 {
                        // EOF
                        // log_info!(worker, "EOF for request");
                        operations.remove(op_index);
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
                            unsafe { sq.push(&write_op)? }
                            sq.sync();
                        } else {
                            log_error!(worker, "got response, but couldn't reach end of request");
                        }
                    }
                }
                Operation::Write(_fd) => {
                    let len = cqe.result();
                    // log_info!(worker, "wrote {}", len);
                }
            }
        }
    }

    Ok(())
}
