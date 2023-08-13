use std::net::SocketAddr;
use std::panic;
use std::process;
use std::sync::Arc;
use std::sync::Barrier;
use std::thread;

use anyhow::Result;

use crate::linux;
use crate::linux::TopologyThread;
use crate::linux::TopologyThreadKind;
use crate::util::*;
use crate::worker;

pub fn start() -> Result<()> {
    let topology = linux::Topology::new(4);

    let addr: SocketAddr = "0.0.0.0:8081".parse()?;
    info!("Starting http-server on {}", addr);

    let mut threads = Vec::with_capacity(topology.threads.len());

    let barrier = Arc::new(Barrier::new(topology.threads.len()));

    for thread in topology.threads.iter().filter(|t| t.kind == TopologyThreadKind::IO) {
        let thread = thread.clone();

        let barrier = barrier.clone();
        let thread = thread::Builder::new()
            .name(format!("httpsrv-io-worker-{}-c{}", thread.worker_id, thread.core))
            .spawn(move || {
                let (thread_id, processor, name) = get_worker_info(&thread);
                let worker = worker::IoWorker::new(thread.worker_id, thread_id, processor, name.clone(), addr);

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

                worker.run(barrier)
            })?;

        threads.push(thread);
    }

    for t in threads {
        t.join()
            .expect("No threads should panic, as process should exit immediately in that case")?;
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
