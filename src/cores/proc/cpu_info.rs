use crate::cores::helper::hack::Hack;
use parking_lot::Mutex;
use serde::{Deserialize, Serialize};
use std::collections::{HashMap, HashSet};
use std::io::BufRead;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::LazyLock;
use std::time::Instant;

static PROC_CPU_CACHE: LazyLock<Mutex<(CpuInfo, Instant, AtomicBool)>> = LazyLock::new(|| {
    Mutex::new((
        CpuInfo::default(),
        Instant::now(),
        AtomicBool::new(true), // boolean identity as initial value
    ))
});

static PROC_LAST_FORCE_INSTANT: LazyLock<Mutex<Instant>> =
    LazyLock::new(|| Mutex::new(Instant::now()));
static PROC_PROCESSING: LazyLock<AtomicBool> = LazyLock::new(|| AtomicBool::new(false));

const PROC_DURATION_IN_SECONDS: u64 = 1;
const PROC_DURATION_RELOAD_SECONDS: u64 = 60;
const DURATION_MIN_FORCE_IN_MS: u128 = 50;

#[derive(Debug, Clone, Default, PartialEq, Eq, Hash, Deserialize, Serialize)]
pub struct Cpu {
    pub processor: usize,
    pub vendor_id: String,
    pub cpu_family: usize,
    pub model: usize,
    pub model_name: String,
    pub stepping: usize,
    pub microcode: String,
    pub cpu_mhz: String,
    pub cpu_hz: usize,
    pub cache_size: usize,
    pub physical_id: usize,
    pub siblings: usize,
    pub core_id: usize,
    pub cpu_cores: usize,
    pub apicid: usize,
    pub initial_apicid: usize,
    pub fpu: bool,
    pub fpu_exception: bool,
    pub cpuid_level: usize,
    pub wp: bool,
    pub flags: Vec<String>,
    pub vmx_flags: Vec<String>,
    pub bugs: Vec<String>,
    pub bogomips: String,
    pub clflush_size: usize,
    pub cache_alignment: usize,
    pub address_sizes: String,
    pub power_management: String,
}

impl Cpu {
    fn set_data<K: AsRef<str>, V: AsRef<str>>(&mut self, index: usize, key: K, value: V) {
        let key = key.as_ref();
        let value = value.as_ref();
        match key.replace(" ", "_").to_lowercase().as_str() {
            "processor" => self.processor = value.parse::<usize>().unwrap_or(index),
            "vendor_id" => self.vendor_id = value.to_string(),
            "cpu_family" => self.cpu_family = value.parse::<usize>().unwrap_or(0),
            "model" => self.model = value.parse::<usize>().unwrap_or(0),
            "model_name" => self.model_name = value.to_string(),
            "stepping" => self.stepping = value.parse::<usize>().unwrap_or(0),
            "microcode" => self.microcode = value.to_string(),
            "cpu_mhz" => {
                self.cpu_mhz = value.to_string();
                self.cpu_hz = (value.parse::<f64>().unwrap_or(0.0) * 1000.0f64) as usize;
            }
            "cache_size" => self.cache_size = Hack::size_to_bytes(&value).unwrap_or(0) as usize,
            "physical_id" => self.physical_id = value.parse::<usize>().unwrap_or(0),
            "siblings" => self.siblings = value.parse::<usize>().unwrap_or(0),
            "core_id" => self.core_id = value.parse::<usize>().unwrap_or(0),
            "cpu_cores" => self.cpu_cores = value.parse::<usize>().unwrap_or(0),
            "apicid" => self.apicid = value.parse::<usize>().unwrap_or(0),
            "initial_apicid" => self.initial_apicid = value.parse::<usize>().unwrap_or(0),
            "fpu" => self.fpu = value == "yes",
            "fpu_exception" => self.fpu_exception = value == "yes",
            "cpuid level" => self.cpuid_level = value.parse::<usize>().unwrap_or(0),
            "wp" => self.wp = value == "yes",
            "flags" => self.flags = value.split_whitespace().map(|s| s.to_string()).collect(),
            "vmx_flags" => {
                self.vmx_flags = value.split_whitespace().map(|s| s.to_string()).collect()
            }
            "bugs" => self.bugs = value.split_whitespace().map(|s| s.to_string()).collect(),
            "bogomips" => self.bogomips = value.to_string(),
            "clflush_size" => self.clflush_size = value.parse::<usize>().unwrap_or(0),
            "cache_alignment" => self.cache_alignment = value.parse::<usize>().unwrap_or(0),
            "address_sizes" => self.address_sizes = value.to_string(),
            "power_management" => self.power_management = value.to_string(),
            _ => {}
        }
    }
}

#[derive(Debug, Clone, Default, PartialEq, Eq, Hash, Deserialize, Serialize)]
pub struct CpuInfo {
    pub cpu: Vec<Cpu>,
}

// /proc/cpuinfo
impl CpuInfo {
    pub fn get() -> Self {
        {
            let lock = PROC_CPU_CACHE.lock();
            let reparse = lock.2.load(Ordering::Acquire)
                || lock.1.elapsed().as_secs() > PROC_DURATION_RELOAD_SECONDS;
            if !reparse {
                return lock.0.clone();
            }
        }
        Self::__force_reparse()
    }
    pub fn total_hertz(&self) -> usize {
        self.cpu.iter().map(|cpu| cpu.cpu_hz).sum()
    }
    pub fn cpu_usage() -> CpuUsage {
        CpuUsage::get()
    }
    pub fn refresh_cpu_usage() {
        CpuUsage::refresh();
    }
    pub fn get_cpu_usage(&self) -> CpuUsage {
        CpuUsage::get()
    }
    pub fn logical_cores(&self) -> usize {
        self.cpu.len()
    }
    pub fn physical_cores(&self) -> usize {
        let mut cores = HashSet::new();
        for cpu in &self.cpu {
            cores.insert((cpu.physical_id, cpu.core_id));
        }
        cores.len()
    }
    pub fn model(&self) -> Option<String> {
        if let Some(first) = self.cpu.first() {
            return Some(first.model_name.clone());
        }
        None
    }
    pub fn model_id(&self) -> Option<String> {
        if let Some(first) = self.cpu.first() {
            return Some(format!("{}-{}", first.cpu_family, first.model));
        }
        None
    }
    pub fn model_name(&self) -> Option<String> {
        if let Some(first) = self.cpu.first() {
            return Some(first.model_name.clone());
        }
        None
    }
    pub fn brand(&self) -> Option<String> {
        if let Some(first) = self.cpu.first() {
            return Some(
                first
                    .model_name
                    .split_whitespace()
                    .collect::<Vec<&str>>()
                    .join(" ")
                    .to_string(),
            );
        }
        None
    }
    pub fn family(&self) -> Option<String> {
        if let Some(first) = self.cpu.first() {
            return Some(format!("{}-{}", first.cpu_family, first.model));
        }
        None
    }
    pub fn cpu_freq_ghz(&self) -> Option<String> {
        if let Some(first) = self.cpu.first() {
            let mhz = first.cpu_mhz.parse::<f64>().unwrap_or(0.0);
            return Some(format!("{:.2} GHz", mhz / 1000.0));
        }
        None
    }

    pub fn refresh() {
        Self::refresh_cpu_usage();
        Self::__force_reparse();
    }

    //noinspection DuplicatedCode
    fn __force_reparse() -> Self {
        if PROC_PROCESSING.swap(true, Ordering::Acquire) {
            {
                let lock = PROC_CPU_CACHE.lock();
                return lock.0.clone();
            }
        }
        let instant = Instant::now();
        {
            let mut last = PROC_LAST_FORCE_INSTANT.lock();
            if last.elapsed().as_millis() < DURATION_MIN_FORCE_IN_MS {
                let lock = PROC_CPU_CACHE.lock();
                if !lock.2.load(Ordering::Acquire) {
                    PROC_PROCESSING.store(false, Ordering::Release);
                    return lock.0.clone();
                }
            }
            *last = instant;
        }

        let mut cpu_set = Vec::new();
        let mut cpu_info = CpuInfo::default();
        if let Ok(file) = std::fs::File::open("/proc/cpuinfo") {
            let reader = std::io::BufReader::new(file);
            for line in reader.lines().flatten() {
                if let Some((key, value)) = line
                    .split_once(':')
                    .map(|(k, v)| (k.trim().to_lowercase(), v.trim()))
                {
                    if key == "processor" {
                        cpu_set.push(Cpu::default());
                    }
                    let len = cpu_set.len();
                    if len > 0
                        && let Some(current_cpu) = cpu_set.last_mut()
                    {
                        current_cpu.set_data(len - 1, key, value);
                    }
                }
            }
            cpu_info.cpu = cpu_set;
            {
                let mut lock = PROC_CPU_CACHE.lock();
                lock.0 = cpu_info.clone();
                lock.1 = Instant::now();
                lock.2.store(false, Ordering::Release);
            }
        }
        PROC_PROCESSING.store(false, Ordering::Release);
        cpu_info
    }
}

static CPU_USAGE_CACHE: LazyLock<Mutex<(CpuUsage, Instant, AtomicBool)>> = LazyLock::new(|| {
    Mutex::new((
        CpuUsage::default(),
        Instant::now(),
        AtomicBool::new(true), // boolean identity as initial value
    ))
});

static CPU_USAGE_LAST_FORCE_INSTANT: LazyLock<Mutex<Instant>> =
    LazyLock::new(|| Mutex::new(Instant::now()));
static CPU_USAGE_PROCESSING: LazyLock<AtomicBool> = LazyLock::new(|| AtomicBool::new(false));
const CPU_DURATION_IN_MS: u128 = 500;

#[derive(Debug, Clone, Default, PartialEq, Eq, Hash, Deserialize, Serialize)]
pub struct CpuLoad {
    // cpu  19169097 19456 2966023 208937354 1754210 0 57521 0 0 0
    pub cpu: usize, // cpu index
    pub total: u64, // total usage
    pub user: u64,
    pub nice: u64,
    pub system: u64,
    pub idle: u64,
    pub iowait: u64,
    pub irq: u64,
    pub softirq: u64,
}

impl CpuLoad {
    pub fn calculate_usage(&self, previous: &CpuLoad) -> f64 {
        let total_delta = self.total.saturating_sub(previous.total);
        if total_delta == 0 {
            return 0.0;
        }
        let idle_delta =
            self.idle.saturating_sub(previous.idle) + self.iowait.saturating_sub(previous.iowait);
        let work_delta = total_delta.saturating_sub(idle_delta);
        (work_delta as f64 / total_delta as f64) * 100.0
    }
    fn set_data(&mut self, index: usize, value: &str) {
        let v: Vec<u64> = value
            .split_whitespace()
            .map(|s| s.parse().unwrap_or(0))
            .collect();

        if v.len() >= 7 {
            self.cpu = index;
            self.user = v[0];
            self.nice = v[1];
            self.system = v[2];
            self.idle = v[3];
            self.iowait = v[4];
            self.irq = v[5];
            self.softirq = v[6];
            // calculate all
            self.total = v.iter().sum();
        }
    }
}

#[derive(Debug, Clone, Default, PartialEq, Eq, Hash, Deserialize, Serialize)]
pub struct CpuUsage {
    pub total: CpuLoad,
    pub cpu: Vec<CpuLoad>,
    pub ctxt: usize,
    pub btime: u64,
    pub processes: usize,
    pub procs_running: usize,
    pub procs_blocked: usize,
    pub softirq: Vec<usize>,
}

impl CpuUsage {
    pub fn get() -> Self {
        {
            let lock = CPU_USAGE_CACHE.lock();
            let reparse =
                lock.2.load(Ordering::Acquire) || lock.1.elapsed().as_millis() > CPU_DURATION_IN_MS;
            if !reparse {
                return lock.0.clone();
            }
        }
        Self::__force_reparse()
    }

    pub fn refresh() {
        Self::__force_reparse();
    }
    fn __force_reparse() -> Self {
        if CPU_USAGE_PROCESSING.swap(true, Ordering::Acquire) {
            {
                let lock = CPU_USAGE_CACHE.lock();
                return lock.0.clone();
            }
        }
        let instant = Instant::now();
        {
            let mut last = CPU_USAGE_LAST_FORCE_INSTANT.lock();
            if last.elapsed().as_millis() < DURATION_MIN_FORCE_IN_MS {
                let lock = CPU_USAGE_CACHE.lock();
                if !lock.2.load(Ordering::Acquire) {
                    CPU_USAGE_PROCESSING.store(false, Ordering::Release);
                    return lock.0.clone();
                }
            }
            *last = instant;
        }

        let mut cpu_total = CpuLoad::default();
        let mut cpu_set = HashMap::new();
        let mut cpu_usage = CpuUsage::default();
        if let Ok(file) = std::fs::File::open("/proc/stat") {
            let reader = std::io::BufReader::new(file);
            for line in reader.lines().flatten() {
                if let Some((key, value)) = line
                    .split_once(' ')
                    .map(|(k, v)| (k.trim().to_lowercase(), v.trim()))
                {
                    match key.as_str() {
                        "ctxt" => cpu_usage.ctxt = value.parse::<usize>().unwrap_or(0),
                        "btime" => cpu_usage.btime = value.parse::<u64>().unwrap_or(0),
                        "processes" => cpu_usage.processes = value.parse::<usize>().unwrap_or(0),
                        "procs_running" => {
                            cpu_usage.procs_running = value.parse::<usize>().unwrap_or(0)
                        }
                        "procs_blocked" => {
                            cpu_usage.procs_blocked = value.parse::<usize>().unwrap_or(0)
                        }
                        "softirq" => {
                            let irq = value.split_whitespace();
                            cpu_usage.softirq = Vec::from_iter(irq)
                                .iter()
                                .map(|s| s.parse::<usize>().unwrap_or(0))
                                .collect::<Vec<usize>>();
                        }
                        "cpu" => {
                            cpu_total.set_data(0, value);
                        }
                        _ => {
                            if !key.starts_with("cpu") {
                                continue;
                            }
                            let cpu_id = key[3..].parse::<isize>().unwrap_or(-1);
                            if cpu_id < 0 {
                                continue;
                            }
                            let cpu_id = cpu_id as usize;
                            let len = cpu_set.len();
                            let current_cpu =
                                cpu_set.entry(cpu_id).or_insert_with(|| CpuLoad::default());
                            current_cpu.set_data(len, value);
                            continue;
                        }
                    }
                }
            }
            let mut sorted_vec: Vec<(&usize, &CpuLoad)> = cpu_set.iter().collect();
            sorted_vec.sort_by_key(|&(key, _)| key);
            // sort
            cpu_usage.cpu = sorted_vec.into_iter().map(|(_, cpu)| cpu.clone()).collect();
            cpu_usage.total = cpu_total;
            {
                let mut lock = CPU_USAGE_CACHE.lock();
                lock.0 = cpu_usage.clone();
                lock.1 = Instant::now();
                lock.2.store(false, Ordering::Release);
            }
        }
        PROC_PROCESSING.store(false, Ordering::Release);
        cpu_usage
    }
}
