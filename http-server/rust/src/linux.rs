use std::{
    collections::{HashMap, HashSet},
    fs::File,
    io::{BufReader, Read},
    mem::{size_of, MaybeUninit},
    sync::{Arc, OnceLock},
};

pub struct CpuInfo {
    pub processors: Vec<Processor>,
    _private: (),
}

pub struct Processor {
    pub processor: usize,
    pub model_name: String,
    pub cache_size: String,
    pub physical_id: usize,
    pub core_id: usize,
    pub cpu_cores: usize,
    pub apicid: usize,
    pub cache_alignment: usize,
    _private: (),
}

static CPUINFO_CACHE: OnceLock<CpuInfo> = OnceLock::new();

pub fn get_cpu_info() -> &'static CpuInfo {
    CPUINFO_CACHE.get_or_init(|| {
        let f = File::open("/proc/cpuinfo").unwrap();
        let mut reader = BufReader::new(f);

        let mut buffer = String::with_capacity(1024 * 32);
        let _size = reader.read_to_string(&mut buffer).unwrap();

        let mut processors: Vec<Processor> = Vec::with_capacity(4);

        let mut current_processor = HashMap::<&str, &str>::new();

        for line in buffer.lines() {
            if line.trim().is_empty() {
                continue;
            }

            let line = line.trim();
            if line.starts_with("processor") {
                if current_processor.len() > 0 {
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

        CpuInfo {
            processors,
            _private: (),
        }
    })
}

pub fn get_physical_core_count() -> u16 {
    let cpu_info = get_cpu_info();

    cpu_info
        .processors
        .iter()
        .map(|v| v.core_id)
        .collect::<HashSet<_>>()
        .len() as u16
}

pub fn pin_thread(processor: usize) {
    let cpu_info = get_cpu_info();
    assert!(
        cpu_info.processors.iter().any(|p| p.processor == processor),
        "processor argument not found in /proc/cpuinfo"
    );

    unsafe {
        let thread = libc::pthread_self();

        let mut cpu_set: MaybeUninit<libc::cpu_set_t> = MaybeUninit::uninit();
        libc::CPU_ZERO(cpu_set.assume_init_mut());
        libc::CPU_SET(processor, cpu_set.assume_init_mut());

        let ret = libc::pthread_setaffinity_np(thread, size_of::<libc::cpu_set_t>(), cpu_set.as_ptr());
        assert!(ret == 0, "thread pinning failed: {ret}");
    }
}

pub fn thread_per_core() -> (u16, Arc<Vec<usize>>) {
    let cpu_info = get_cpu_info();
    let core_count = get_physical_core_count() / 2;
    let mut processors_to_use = HashMap::<usize, usize>::with_capacity(core_count as usize);
    cpu_info
        .processors
        .iter()
        .for_each(|p| _ = processors_to_use.insert(p.core_id, p.processor));

    let processors_to_use = Arc::new(processors_to_use.values().cloned().collect::<Vec<_>>());
    (core_count, processors_to_use)
}
