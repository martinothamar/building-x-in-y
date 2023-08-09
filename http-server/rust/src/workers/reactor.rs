use anyhow::{Context, Result};
use futures_core::task::__internal::AtomicWaker;
use io_uring::{cqueue, opcode, squeue, types, IoUring};
use std::{
    cell::{Ref, RefCell},
    future::Future,
    os::fd::AsRawFd,
    rc::Rc,
    sync::{
        atomic::{AtomicBool, AtomicI32, Ordering},
        Arc, Barrier,
    },
};

use crate::linux::{Topology, TopologyThreadKind};
use crate::util::*;

#[derive(Clone, Debug)]
pub struct ReactorWorker {
    state: Rc<RefCell<ReactorWorkerState>>,
    shared: Arc<ReactorWorkerShared>,
}

#[derive(Debug)]
struct ReactorWorkerState {
    _worker_id: u16,
    _thread_id: u64,
    _processor: u16,
    name: String,
}

#[derive(Debug)]
pub struct ReactorWorkerShared {
    readiness: Vec<Readiness>,
}

impl ReactorWorker {
    pub fn new(worker_id: u16, thread_id: u64, processor: u16, name: String, topology: Topology) -> Self {
        assert!(topology
            .threads
            .iter()
            .enumerate()
            .all(|(i, t)| t.worker_id as usize == i));

        Self {
            state: Rc::new(RefCell::new(ReactorWorkerState {
                _worker_id: worker_id,
                _thread_id: thread_id,
                _processor: processor,
                name,
            })),
            shared: Arc::new(ReactorWorkerShared {
                readiness: topology
                    .threads
                    .iter()
                    .filter(|t| t.kind == TopologyThreadKind::IO)
                    .map(|_| Readiness::new())
                    .collect(),
            }),
        }
    }

    pub fn get_shared(&self) -> Arc<ReactorWorkerShared> {
        Arc::clone(&self.shared)
    }

    pub fn run(self, barrier: Arc<Barrier>) -> Result<()> {
        let mut ring = IoUring::<squeue::Entry, cqueue::Entry>::builder()
            .setup_coop_taskrun()
            // .setup_defer_taskrun()
            // .setup_iopoll()
            .setup_single_issuer()
            .setup_cqsize(64)
            // .setup_sqpoll(100)
            // .setup_sqpoll_cpu((worker.borrow().processor + 1) as u32)
            .build(32)
            .context("failed to initialize IO uring")?;

        barrier.wait(); // All fd should be set for readiness

        let shared = self.get_shared();

        let fds = shared
            .readiness
            .iter()
            .map(|v| (v.inner.fd.load(Ordering::SeqCst), v))
            .collect::<Vec<_>>();
        assert!(
            fds.iter().all(|fd| fd.0 != -1 && fd.0 >= 0),
            "All ring fd's should be set from the IO threads"
        );

        let (submitter, mut sq, mut cq) = ring.split();

        for (i, &(fd, _)) in fds.iter().enumerate() {
            let poll_op = opcode::PollAdd::new(types::Fd(fd), libc::POLLIN as _)
                .multi(true) // TODO: hmm multi does not actually work
                .build()
                .user_data(i as u64);

            log_info!(self, "submitting fd, op: {:?}", poll_op);
            unsafe { sq.push(&poll_op)? };
        }
        sq.sync();

        let mut state: Vec<(bool, u32)> = vec![(false, 0); fds.len()];

        loop {
            // info!("reactor waiting..");
            submitter.submit_and_wait(1)?;
            cq.sync();

            while !cq.is_empty() {
                for cqe in &mut cq {
                    let i = cqe.user_data() as usize;

                    let flags = cqe.flags();
                    unsafe {
                        let state = state.get_unchecked_mut(i);
                        *state = (true, state.1 | flags);
                    }
                }

                cq.sync();
            }

            for i in 0..state.len() {
                let state = unsafe { state.get_unchecked_mut(i) };
                if !state.0 {
                    continue;
                }

                let &(fd, readiness) = unsafe { fds.get_unchecked(i) };
                let flags = state.1;

                // log_info!(self, "notifying readiness, flags: {}", flags);
                readiness.set_ready();

                if !cqueue::more(flags) {
                    let poll_op = opcode::PollAdd::new(types::Fd(fd), libc::POLLIN as _)
                        .multi(true)
                        .build()
                        .user_data(i as u64);
                    unsafe { sq.push(&poll_op)? };
                    sq.sync();
                }

                *state = (false, 0);
            }
        }

        #[allow(unreachable_code)]
        Ok(())
    }

    #[inline]
    pub fn name(&self) -> Ref<'_, str> {
        Ref::map(self.state.borrow(), |i| i.name.as_str())
    }
}

impl ReactorWorkerShared {
    pub fn readiness(&self, worker_id: u16) -> Readiness {
        unsafe {
            let readiness = self.readiness.get_unchecked(worker_id as usize);
            readiness.clone()
        }
    }
}

#[derive(Clone, Debug)]
pub struct Readiness {
    inner: Arc<ReadinessImpl>,
}

#[derive(Debug)]
#[repr(align(64))]
struct ReadinessImpl {
    waker: AtomicWaker,
    ready: AtomicBool,
    fd: AtomicI32,
}

#[derive(Debug)]
pub struct ReadyFut(Readiness);

impl Future for ReadyFut {
    type Output = ();

    fn poll(self: std::pin::Pin<&mut Self>, cx: &mut std::task::Context<'_>) -> std::task::Poll<Self::Output> {
        self.0.inner.waker.register(cx.waker());

        let ready = self.0.inner.ready.load(Ordering::Acquire);
        match ready {
            true => std::task::Poll::Ready(()),
            false => std::task::Poll::Pending,
        }
    }
}

impl Readiness {
    pub fn new() -> Self {
        Self {
            inner: Arc::new(ReadinessImpl {
                ready: AtomicBool::new(false),
                waker: AtomicWaker::new(),
                fd: AtomicI32::new(-1),
            }),
        }
    }

    fn set_ready(&self) {
        self.inner.ready.store(true, Ordering::Release);
        let waker = self.inner.waker.take();
        let Some(waker) = waker else {
            return;
        };

        waker.wake();
    }

    pub fn set_fd<T: AsRawFd>(&self, fd: &T) {
        self.inner.fd.store(fd.as_raw_fd(), Ordering::SeqCst);
    }

    pub fn wait_readable(&self) -> ReadyFut {
        ReadyFut(self.clone())
    }

    pub fn reset(&self) {
        self.inner.ready.store(false, Ordering::Release);
        _ = self.inner.waker.take()
    }
}
