use anyhow::{Context, Result};
use futures_core::task::__internal::AtomicWaker;
use io_uring::IoUring;
use std::{
    cell::{Ref, RefCell},
    future::Future,
    rc::Rc,
    sync::{
        atomic::{AtomicBool, AtomicPtr, Ordering},
        Arc,
    },
    task::Waker,
};

use crate::linux::{Topology, TopologyThreadKind};

#[derive(Clone, Debug)]
pub struct ReactorWorker {
    state: Rc<RefCell<ReactorWorkerState>>,
    shared: Arc<ReactorWorkerShared>,
}

#[derive(Debug)]
struct ReactorWorkerState {
    _worker_id: u16,
    _thread_id: u64,
    processor: u16,
    name: String,
}

#[derive(Debug)]
pub struct ReactorWorkerShared {
    readiness: Vec<Readiness>,
}

impl ReactorWorker {
    pub fn new(worker_id: u16, thread_id: u64, processor: u16, name: String, topology: Topology) -> Self {
        Self {
            state: Rc::new(RefCell::new(ReactorWorkerState {
                _worker_id: worker_id,
                _thread_id: thread_id,
                processor,
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

    pub fn run(self) -> Result<()> {
        // let ring = IoUring::builder()
        //     .setup_coop_taskrun()
        //     // .setup_defer_taskrun()
        //     .setup_single_issuer()
        //     .setup_cqsize(256)
        //     // .setup_sqpoll(100)
        //     // .setup_sqpoll_cpu((worker.borrow().processor + 1) as u32)
        //     .build(128)
        //     .context("failed to initialize IO uring")?;

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

    pub fn wait_readable(&self) -> ReadyFut {
        ReadyFut(self.clone())
    }

    pub fn reset(&self) {
        self.inner.ready.store(false, Ordering::Release);
        _ = self.inner.waker.take()
    }
}
