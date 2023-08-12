#![allow(clippy::manual_non_exhaustive)]

use std::{
    collections::{BTreeMap, HashMap},
    fs::File,
    io::{BufReader, Read},
    mem::{size_of, MaybeUninit},
    sync::OnceLock,
};

pub struct CpuInfo {
    pub processors: Vec<Processor>,
    pub cores: BTreeMap<u16, Vec<Processor>>,
    _private: (),
}

#[derive(Clone)]
pub struct Processor {
    pub processor: u16,
    pub model_name: String,
    pub cache_size: String,
    pub physical_id: u16,
    pub core_id: u16,
    pub cpu_cores: u16,
    pub apicid: u16,
    pub cache_alignment: u16,
    _private: (),
}

static CPUINFO_CACHE: OnceLock<CpuInfo> = OnceLock::new();

pub fn get_cpu_info() -> &'static CpuInfo {
    CPUINFO_CACHE.get_or_init(|| {
        let f = File::open("/proc/cpuinfo").unwrap();
        let mut reader = BufReader::new(f);

        let mut buffer = String::with_capacity(1024 * 32);
        let _size = reader.read_to_string(&mut buffer).unwrap();

        let mut processors: Vec<Processor> = Vec::with_capacity(8);
        let mut cores: BTreeMap<u16, Vec<Processor>> = BTreeMap::new();

        let mut current_processor = HashMap::<&str, &str>::new();

        for line in buffer.lines() {
            if line.trim().is_empty() {
                continue;
            }

            let line = line.trim();
            if line.starts_with("processor") {
                if !current_processor.is_empty() {
                    processors.push(Processor {
                        processor: current_processor.get("processor").unwrap().parse().unwrap(),
                        model_name: current_processor.get("model name").unwrap().to_string(),
                        cache_size: current_processor.get("cache size").unwrap().to_string(),
                        physical_id: current_processor.get("physical id").unwrap().parse().unwrap(),
                        core_id: current_processor.get("core id").unwrap().parse().unwrap(),
                        cpu_cores: current_processor.get("cpu cores").unwrap().parse().unwrap(),
                        apicid: current_processor.get("apicid").unwrap().parse().unwrap(),
                        cache_alignment: current_processor.get("cache_alignment").unwrap().parse().unwrap(),
                        _private: (),
                    });
                }
                current_processor.clear();
            }

            let (k, v) = line.split_once(':').map(|(k, v)| (k.trim(), v.trim())).unwrap();
            _ = current_processor.insert(k, v);
        }

        for processor in processors.iter() {
            let core = cores.entry(processor.core_id).or_default();
            core.push(processor.clone());
        }

        CpuInfo {
            processors,
            cores,
            _private: (),
        }
    })
}

pub fn pin_thread(processor: u16) {
    let cpu_info = get_cpu_info();
    assert!(
        cpu_info.processors.iter().any(|p| p.processor == processor),
        "processor argument not found in /proc/cpuinfo"
    );

    unsafe {
        let thread = libc::pthread_self();

        let mut cpu_set: MaybeUninit<libc::cpu_set_t> = MaybeUninit::uninit();
        libc::CPU_ZERO(cpu_set.assume_init_mut());
        libc::CPU_SET(processor as usize, cpu_set.assume_init_mut());

        let ret = libc::pthread_setaffinity_np(thread, size_of::<libc::cpu_set_t>(), cpu_set.as_ptr());
        assert!(ret == 0, "thread pinning failed: {ret}");
    }
}

#[derive(Clone, Debug)]
pub struct Topology {
    pub core_count: u16,
    pub processor_count: u16,
    pub threads: Vec<TopologyThread>,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum TopologyThreadKind {
    IO,
}

#[derive(Clone, Debug)]
pub struct TopologyThread {
    pub worker_id: u16,
    pub core: u16,
    pub processor: u16,
    pub kind: TopologyThreadKind,
}

impl Topology {
    pub fn new() -> Self {
        let cpu_info = get_cpu_info();
        let core_count = cpu_info.cores.len();
        assert!(core_count > 2 && core_count % 2 == 0);
        let io_cores = core_count / 2;

        let mut threads = Vec::with_capacity(io_cores + 1);

        for (worker_id, core) in cpu_info.cores.iter().take(io_cores).enumerate() {
            threads.push(TopologyThread {
                worker_id: worker_id as u16,
                core: *core.0,
                processor: core.1[0].processor,
                kind: TopologyThreadKind::IO,
            });
        }

        Self {
            core_count: core_count as u16,
            processor_count: core_count as u16,
            threads,
        }
    }
}
