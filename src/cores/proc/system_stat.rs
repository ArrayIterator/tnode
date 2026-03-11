use parking_lot::Mutex;
use serde::{Deserialize, Serialize};
use std::io::BufRead;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::LazyLock;
use std::time::{Duration, Instant, SystemTime, UNIX_EPOCH};

static STATE_USED: LazyLock<Mutex<(SystemStat, SystemStat, Instant)>> = LazyLock::new(|| {
    Mutex::new({ (SystemStat::default(), SystemStat::default(), Instant::now()) })
});

const DURATION_CHECK: Duration = Duration::from_millis(500);
static PROC_PROCESSING: LazyLock<AtomicBool> = LazyLock::new(|| AtomicBool::new(false));

#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct CpuMetric {
    pub name: String,
    pub user: u64,
    pub nice: u64,
    pub system: u64,
    pub idle: u64,
    pub iowait: u64,
    pub irq: u64,
    pub softirq: u64,
    pub steal: u64,
    pub total: u64,
}

#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct SystemStat {
    pub cpu_total: CpuMetric,
    pub cores: Vec<CpuMetric>,
    pub boot_time: u64,        // btime
    pub context_switches: u64, // ctxt
    pub processes_total: u64,  // processes
    pub procs_running: u32,    // procs_running
    pub procs_blocked: u32,    // procs_blocked
}

impl SystemStat {
    pub fn percentage_cpu_usage(duration: Duration) -> f64 {
        let need_refresh = {
            let lock = STATE_USED.lock();
            lock.0.boot_time == 0 || lock.2.elapsed() >= duration
        };
        let (current, previous) = if need_refresh {
            Self::refresh()
        } else {
            let lock = STATE_USED.lock();
            (lock.0.clone(), lock.1.clone())
        };
        current.get_cpu_usage(&previous)
    }

    pub fn get_cpu_usage(&self, prev: &SystemStat) -> f64 {
        self.calculate_percent(&self.cpu_total, &prev.cpu_total)
    }

    pub fn get_uptime(&self) -> u64 {
        let now = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap_or_default()
            .as_secs();
        if self.boot_time == 0 {
            return 0;
        }
        now.saturating_sub(self.boot_time)
    }

    fn calculate_percent(&self, curr: &CpuMetric, prev: &CpuMetric) -> f64 {
        let total_delta = curr.total.saturating_sub(prev.total);
        if total_delta == 0 {
            return 0.0;
        }

        let idle_delta = (curr.idle + curr.iowait).saturating_sub(prev.idle + prev.iowait);
        let used_delta = total_delta.saturating_sub(idle_delta);

        (used_delta as f64 / total_delta as f64) * 100.0
    }

    pub fn refresh() -> (SystemStat, SystemStat) {
        if PROC_PROCESSING.swap(true, Ordering::Acquire) {
            {
                let lock = STATE_USED.lock();
                return (lock.0.clone(), lock.1.clone());
            }
        }

        let mut next_stat = SystemStat::default();
        if let Ok(file) = std::fs::File::open("/proc/stat") {
            let reader = std::io::BufReader::new(file);
            for line in reader.lines().flatten() {
                let parts: Vec<&str> = line.split_whitespace().collect();
                if parts.is_empty() {
                    continue;
                }

                match parts[0] {
                    "ctxt" => next_stat.context_switches = parts[1].parse().unwrap_or(0),
                    "btime" => next_stat.boot_time = parts[1].parse().unwrap_or(0),
                    "processes" => next_stat.processes_total = parts[1].parse().unwrap_or(0),
                    "procs_running" => next_stat.procs_running = parts[1].parse().unwrap_or(0),
                    "procs_blocked" => next_stat.procs_blocked = parts[1].parse().unwrap_or(0),
                    key if key.starts_with("cpu") => {
                        let mut m = CpuMetric {
                            name: key.to_string(),
                            user: parts.get(1).and_then(|v| v.parse().ok()).unwrap_or(0),
                            nice: parts.get(2).and_then(|v| v.parse().ok()).unwrap_or(0),
                            system: parts.get(3).and_then(|v| v.parse().ok()).unwrap_or(0),
                            idle: parts.get(4).and_then(|v| v.parse().ok()).unwrap_or(0),
                            iowait: parts.get(5).and_then(|v| v.parse().ok()).unwrap_or(0),
                            irq: parts.get(6).and_then(|v| v.parse().ok()).unwrap_or(0),
                            softirq: parts.get(7).and_then(|v| v.parse().ok()).unwrap_or(0),
                            steal: parts.get(8).and_then(|v| v.parse().ok()).unwrap_or(0),
                            total: 0,
                        };
                        m.total = m.user
                            + m.nice
                            + m.system
                            + m.idle
                            + m.iowait
                            + m.irq
                            + m.softirq
                            + m.steal;

                        if key == "cpu" {
                            next_stat.cpu_total = m;
                        } else {
                            next_stat.cores.push(m);
                        }
                    }
                    _ => {}
                }
            }
            {
                PROC_PROCESSING.store(false, Ordering::Release);
                let mut lock = STATE_USED.lock();
                lock.1 = lock.0.clone();
                lock.0 = next_stat;
                lock.2 = Instant::now();
                let res = (lock.0.clone(), lock.1.clone());
                res
            }
        } else {
            {
                PROC_PROCESSING.store(false, Ordering::Release);
                let mut lock = STATE_USED.lock();
                let res = (lock.0.clone(), lock.1.clone());
                res
            }
        }
    }
}
