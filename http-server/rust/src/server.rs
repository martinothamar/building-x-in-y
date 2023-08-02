use std::panic;
use std::process;
use std::sync::Arc;
use std::sync::OnceLock;
use std::thread;
use std::time::Duration;

use anyhow::anyhow;
use anyhow::Result;

use crate::linux;
use crate::linux::TopologyThread;
use crate::linux::TopologyThreadKind;
use crate::util::*;
use crate::workers::io;
use crate::workers::reactor;

pub fn start() -> Result<()> {
    let topology = linux::Topology::new();

    let mut threads = Vec::with_capacity(topology.threads.len());

    let shared_reactor = Arc::new(OnceLock::new());

    {
        let topology = topology.clone();
        let thread = topology
            .threads
            .iter()
            .cloned()
            .find(|t| t.kind == TopologyThreadKind::Reactor)
            .unwrap();

        let reactor_cell = shared_reactor.clone();
        let thread = thread::Builder::new()
            .name(format!("httpsrv-reactor-worker-{}-c{}", thread.worker_id, thread.core))
            .spawn(move || {
                let (thread_id, processor, name) = get_worker_info(&thread);
                let worker =
                    reactor::ReactorWorker::new(thread.worker_id, thread_id, processor, name.clone(), topology);

                reactor_cell
                    .set(worker.get_shared())
                    .map_err(|_| anyhow!("Couldn't initalize shared reactor"))?;

                {
                    let name = name.clone();
                    let orig_hook = panic::take_hook();
                    panic::set_hook(Box::new(move |panic_info| {
                        orig_hook(panic_info);
                        error!("[{}] reactor thread panicked: {}", &name, panic_info);
                        process::exit(1);
                    }));
                }

                log_info!(worker, "reactor thread starting");
                linux::pin_thread(processor);
                log_info!(worker, "thread pinned to processor: {}", processor);

                worker.run()
            })?;

        threads.push(thread);
    }

    for _ in 0..100 {
        if shared_reactor.get().is_none() {
            std::thread::sleep(Duration::from_millis(1));
        } else {
            break;
        }
    }

    let Some(shared_reactor) = shared_reactor.get() else {
        error!("error initializing shared reactor, timed out after 100ms");
        process::exit(2);
    };

    for thread in topology.threads.iter().filter(|t| t.kind == TopologyThreadKind::IO) {
        let thread = thread.clone();
        let shared_reactor = shared_reactor.clone();

        let thread = thread::Builder::new()
            .name(format!("httpsrv-io-worker-{}-c{}", thread.worker_id, thread.core))
            .spawn(move || {
                let (thread_id, processor, name) = get_worker_info(&thread);
                let readiness = shared_reactor.readiness(thread.worker_id);
                let worker = io::IoWorker::new(thread.worker_id, thread_id, processor, name.clone(), readiness);

                {
                    let name = name.clone();
                    let orig_hook = panic::take_hook();
                    panic::set_hook(Box::new(move |panic_info| {
                        orig_hook(panic_info);
                        error!("[{}] IO thread panicked: {}", &name, panic_info);
                        process::exit(1);
                    }));
                }

                log_info!(worker, "IO thread starting");
                linux::pin_thread(processor);
                log_info!(worker, "thread pinned to processor: {}", processor);

                worker.run()
            })?;

        threads.push(thread);
    }

    for t in threads {
        t.join().map_err(|e| anyhow!("Thread panicked: {:?}", e))??;
    }

    Ok(())
}

fn get_worker_info(thread: &TopologyThread) -> (u64, u16, String) {
    let os_thread = thread::current();
    let name = os_thread.name().unwrap().to_owned();
    let thread_id = unsafe { libc::pthread_self() };
    let processor = thread.processor;
    (thread_id, processor, name)
}
