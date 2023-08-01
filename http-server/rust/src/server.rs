use std::thread;

use anyhow::anyhow;
use anyhow::Result;

use crate::linux;
use crate::util::*;
use crate::workers::io;

pub fn start() -> Result<()> {
    let toplogy = linux::Topology::new();

    let mut threads = Vec::with_capacity(toplogy.threads.len());

    for thread in toplogy.threads {
        let thread = thread.clone();

        let thread = match thread.kind {
            linux::TopologyThreadKind::IO => thread::Builder::new()
                .name(format!("httpsrv-worker-{}", thread.core))
                .spawn(move || {
                    let os_thread = thread::current();
                    let name = os_thread.name().unwrap().to_owned();
                    let id = unsafe { libc::pthread_self() };
                    let processor = thread.processor;
                    let worker = io::IoWorker::new(id, processor, name);

                    log_info!(worker, "thread starting");

                    linux::pin_thread(processor);
                    log_info!(worker, "thread pinned to processor: {}", processor);

                    worker.run()
                })?,
            linux::TopologyThreadKind::Reactor => thread::Builder::new()
                .name(format!("httpsrv-reactor-{}", thread.core))
                .spawn(move || {
                    let os_thread = thread::current();
                    let name = os_thread.name().unwrap().to_owned();
                    let id = unsafe { libc::pthread_self() };
                    let processor = thread.processor;

                    Ok(())
                })?,
        };

        threads.push(thread);
    }

    for t in threads {
        t.join().map_err(|e| anyhow!("Thread panicked: {:?}", e))??;
    }

    Ok(())
}

