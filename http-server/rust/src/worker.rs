use std::cell::Ref;
use std::cell::RefCell;
use std::io;
use std::net::SocketAddr;
use std::net::TcpListener;
use std::os::fd::AsRawFd;
use std::rc::Rc;
use std::sync::Arc;
use std::sync::Barrier;

use anyhow::anyhow;
use anyhow::Context;
use anyhow::Result;
use io_uring::opcode;
use io_uring::types;
use io_uring::IoUring;
use slab::Slab;
use socket2::Domain;
use socket2::Socket;
use socket2::Type;

use crate::buf_ring;

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
    addr: SocketAddr,

    active_connections: usize,
}

impl IoWorker {
    pub fn new(worker_id: u16, thread_id: u64, processor: u16, name: String, addr: SocketAddr) -> Self {
        Self {
            inner: Rc::new(RefCell::new(IoWorkerImpl {
                _worker_id: worker_id,
                _thread_id: thread_id,
                processor,
                name,
                addr,

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
    fn _active_connections(&self) -> usize {
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

    pub fn run(self, barrier: Arc<Barrier>) -> Result<()> {
        let ring = IoUring::builder()
            .setup_coop_taskrun()
            // .setup_defer_taskrun()
            .setup_single_issuer()
            .setup_cqsize(1024)
            // .setup_sqpoll(100)
            // .setup_sqpoll_cpu((worker.borrow().processor + 1) as u32)
            .build(512)
            .context("failed to initialize IO uring")?;

        barrier.wait();

        self.event_loop(ring)
    }

    fn event_loop(self, mut ring: IoUring) -> Result<()> {
        let buf_ring = self.register_buffer_rings(&mut ring)?;
        let bg_id = self.bg_id();

        let mut operations: Slab<Operation> = Slab::with_capacity(1024);

        let listener = create_tcp_listener(self.inner.borrow().addr).context("failed to create TCP listener")?;
        let listener_fd = types::Fd(listener.as_raw_fd());

        let (submitter, mut sq, mut cq) = ring.split();

        {
            let accept_op = opcode::AcceptMulti::new(listener_fd);
            let listener_entry = accept_op.build().user_data(operations.insert(Operation::Accept) as _);
            unsafe { sq.push(&listener_entry) }?;
            sq.sync();
            submitter.submit()?;
        }

        loop {
            // log_info!(self, "waiting for cq");

            submitter.submit_and_wait(1)?;
            cq.sync();

            // log_info!(worker, "reading cqe's");

            while !cq.is_empty() {
                for cqe in &mut cq {
                    let ret = cqe.result();

                    let op_index = cqe.user_data() as usize;
                    let op = &operations[op_index];

                    // log_info!(self, "cqe received: op={:?}, ret={}", op, ret);

                    if ret < 0 {
                        let e = io::Error::from_raw_os_error(-ret);
                        let e: Result<(), io::Error> = match op {
                            Operation::Read(_) if -ret == 104 => {
                                log_info!(self, "client dropped");
                                Ok(())
                            }
                            _ => return Err(e.into()),
                        };
                        if let Err(e) = e {
                            log_error!(self, "error cqe: {}, {:?}", e, op);
                        }
                        continue;
                    }

                    match op {
                        Operation::None => unreachable!(),
                        Operation::Accept => {
                            // log_info!(self, "accepted request");
                            let fd = ret;
                            let read_op = opcode::RecvMulti::new(types::Fd(fd), bg_id)
                                .build()
                                .user_data(operations.insert(Operation::Read(fd)) as _);
                            unsafe { sq.push(&read_op)? };
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
                                let flags = cqe.flags();
                                let buf = match buf_ring.rc.get_buf(buf_ring.clone(), ret as u32, flags) {
                                    Ok(buf) => buf,
                                    Err(e) => {
                                        return Err(anyhow!(
                                            "error getting provided ringbuf. ret={ret}, len={len}, flags={flags}, orig_err={e}"
                                        ));
                                    }
                                };
                                let buf = buf.as_slice();

                                if len >= 4 && buf[len - 4..len].eq(b"\r\n\r\n") {
                                    let write_op = opcode::SendZc::new(
                                        types::Fd(*fd),
                                        resp::RESPONSE_HELLO_WORLD.as_ptr(),
                                        resp::RESPONSE_HELLO_WORLD.len() as u32,
                                    )
                                    .build()
                                    .user_data(operations.insert(Operation::Write(*fd)) as _);
                                    unsafe { sq.push(&write_op)? };
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

                sq.sync();
                cq.sync();
            }
        }

        #[allow(unreachable_code)]
        Ok(())
    }

    fn register_buffer_rings(&self, ring: &mut IoUring) -> Result<FixedSizeBufRing> {
        const RING_ENTRIES: u16 = 4096;
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

fn create_tcp_listener(addr: SocketAddr) -> Result<TcpListener> {
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
