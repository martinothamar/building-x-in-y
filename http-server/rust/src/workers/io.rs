use std::cell::Ref;
use std::cell::RefCell;
use std::io;
use std::net::SocketAddr;
use std::net::TcpListener;
use std::os::fd::AsRawFd;
use std::os::fd::RawFd;
use std::rc::Rc;
use std::sync::Arc;
use std::sync::Barrier;
use std::time::Duration;

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
use tokio::runtime;

use crate::buf_ring;

use super::reactor;
use crate::buf_ring::FixedSizeBufRing;
use crate::resp;
use crate::util::*;

#[derive(Clone)]
pub struct IoWorker {
    inner: Rc<RefCell<IoWorkerImpl>>,
}

struct IoWorkerImpl {
    _worker_id: u16,
    _thread_id: u64,
    processor: u16,
    name: String,

    readiness: reactor::Readiness,

    active_connections: usize,
}

impl IoWorker {
    pub fn new(worker_id: u16, thread_id: u64, processor: u16, name: String, readiness: reactor::Readiness) -> Self {
        Self {
            inner: Rc::new(RefCell::new(IoWorkerImpl {
                _worker_id: worker_id,
                _thread_id: thread_id,
                processor,
                name,

                readiness,

                active_connections: 0,
            })),
        }
    }

    #[inline]
    pub fn processor(&self) -> u16 {
        self.inner.borrow().processor
    }

    #[inline]
    pub fn name(&self) -> Ref<'_, str> {
        Ref::map(self.inner.borrow(), |i| i.name.as_str())
    }

    #[inline]
    fn bg_id(&self) -> u16 {
        self.inner.borrow().processor
    }

    #[inline]
    fn active_connections(&self) -> usize {
        self.inner.borrow().active_connections
    }

    #[inline]
    fn increment_active_connections(&self) -> usize {
        let mut me = self.inner.borrow_mut();
        me.active_connections += 1;
        me.active_connections
    }

    #[inline]
    fn decrement_active_connections(&self) -> usize {
        let mut me = self.inner.borrow_mut();
        me.active_connections -= 1;
        me.active_connections
    }

    #[inline]
    fn wait_readable(&self) -> reactor::ReadyFut {
        let me = self.inner.borrow();
        me.readiness.wait_readable()
    }

    #[inline]
    fn reset_readiness(&self) {
        let me = self.inner.borrow();
        me.readiness.reset();
    }

    #[inline]
    fn send_fd_to_reactor(&self, ring: &IoUring) {
        let me = self.inner.borrow_mut();
        me.readiness.set_fd(ring);
    }

    pub fn run(self, barrier: Arc<Barrier>) -> Result<()> {
        let rt = runtime::Builder::new_current_thread()
            .enable_time()
            // .enable_io()
            .build()
            .context("failed to create async executor")?;
        let local = tokio::task::LocalSet::new();

        let ring = IoUring::builder()
            .setup_coop_taskrun()
            // .setup_defer_taskrun()
            .setup_single_issuer()
            .setup_cqsize(256)
            // .setup_sqpoll(100)
            // .setup_sqpoll_cpu((worker.borrow().processor + 1) as u32)
            .build(128)
            .context("failed to initialize IO uring")?;

        self.send_fd_to_reactor(&ring);

        barrier.wait();

        local.spawn_local(self.event_loop(ring));

        rt.block_on(local);

        #[allow(unreachable_code)]
        Ok(())
    }

    async fn event_loop(self, mut ring: IoUring) -> Result<()> {
        let buf_ring = self.register_buffer_rings(&mut ring)?;
        let bg_id = self.bg_id();

        let mut operations: Slab<Operation> = Slab::with_capacity(64);

        let listener = create_tcp_listener().context("failed to create TCP listener")?;
        let listener_fd = types::Fd(listener.as_raw_fd());

        let ring = RingHandle {
            inner: Rc::new(RefCell::new(ring)),
        };

        ring.register_files(&[listener.as_raw_fd()])?;

        {
            let accept_op = opcode::AcceptMulti::new(listener_fd);
            let listener_entry = accept_op.build().user_data(operations.insert(Operation::Accept) as _);

            ring.push_sq(&listener_entry)
                .context("failed to enqueue socket accept operation to IO uring")?;
            ring.sync_sq();
            ring.submit()?;
        }

        {
            let worker = self.clone();
            tokio::task::spawn_local(async move {
                loop {
                    tokio::time::sleep(Duration::from_secs(5)).await;
                    log_info!(worker, "active connections: {}", worker.active_connections());
                }
            });
        };

        loop {
            // log_info!(self, "waiting for cq");
            self.wait_readable().await;

            // log_info!(worker, "reading cqe's");

            ring.sync_cq();
            for cqe in &ring {
                let ret = cqe.result();

                let op_index = cqe.user_data() as usize;
                let op = &operations[op_index];

                // log_info!(worker, "cqe received: op={:?}, ret={}", op, ret);

                if ret < 0 {
                    let e = io::Error::from_raw_os_error(-ret);
                    log_error!(self, "error cqe: {}, {:?}", e, op);
                    return Err(e.into());
                }

                match op {
                    Operation::None => unreachable!(),
                    Operation::Accept => {
                        // log_info!(self, "accepted request");
                        let fd = ret;
                        let read_op = opcode::RecvMulti::new(types::Fd(fd), bg_id)
                            .build()
                            .user_data(operations.insert(Operation::Read(fd)) as _);
                        ring.push_sq(&read_op)?;
                        self.increment_active_connections();
                    }
                    Operation::Read(fd) => {
                        if ret == 0 {
                            // EOF
                            // log_info!(self, "EOF for request");
                            operations.remove(op_index);
                            self.decrement_active_connections();
                        } else {
                            let len = ret as usize;
                            let result = ret as usize;
                            assert_eq!(result, len);
                            let flags = cqe.flags();
                            let buf = buf_ring.rc.get_buf(buf_ring.clone(), result as u32, flags)?;
                            let buf = buf.as_slice();

                            if len >= 4 && buf[len - 4..len].eq(b"\r\n\r\n") {
                                let write_op = opcode::SendZc::new(
                                    types::Fd(*fd),
                                    resp::RESPONSE_HELLO_WORLD.as_ptr(),
                                    resp::RESPONSE_HELLO_WORLD.len() as u32,
                                )
                                .build()
                                .user_data(operations.insert(Operation::Write(*fd)) as _);
                                ring.push_sq(&write_op)?;
                                // log_info!(self, "submitted write");
                            } else {
                                log_error!(self, "got response, but couldn't reach end of request");
                            }
                        }
                    }
                    Operation::Write(_fd) => {
                        // log_info!(self, "wrote {}", _fd);
                    }
                }
            }
            ring.sync_cq();

            // log_info!(self, "syncing sq");
            ring.sync_sq();

            self.reset_readiness();
            ring.submit()?;
        }

        #[allow(unreachable_code)]
        Ok(())
    }

    fn register_buffer_rings(&self, ring: &mut IoUring) -> Result<FixedSizeBufRing> {
        const RING_ENTRIES: u16 = 128;
        const PAGE_SIZE: usize = 4096;
        let bg_id = self.processor();

        let buf_ring = buf_ring::Builder::new(bg_id)
            .ring_entries(RING_ENTRIES)
            .buf_len(PAGE_SIZE)
            .build()?;
        buf_ring.rc.register(ring)?;

        Ok(buf_ring)
    }
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
