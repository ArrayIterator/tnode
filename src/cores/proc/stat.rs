use nix::libc;
use parking_lot::Mutex;
use serde::{Deserialize, Serialize};
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::LazyLock;
use std::time::{Duration, Instant};

#[derive(Debug, Default, Serialize, Deserialize, Clone, PartialEq)]
pub struct Stat {
    pub pid: i32,
    pub comm: String,
    pub state: String,
    pub vsize: usize, // Virtual Size (Bytes)
    pub rss: usize,   // Resident Set Size (Bytes)
    pub utime: u64,
    pub stime: u64,
    pub cpu_usage: f64,
}

static PROC_STATE_CACHE: LazyLock<Mutex<(Stat, Instant, AtomicBool)>> = LazyLock::new(|| {
    let mut initial_stat = Stat::default();
    if let Ok(content) = std::fs::read_to_string("/proc/self/stat") {
        if let Some(last_paren) = content.rfind(')') {
            let page_size = unsafe { libc::sysconf(libc::_SC_PAGESIZE) as usize };
            let (head, tail) = content.split_at(last_paren + 1);

            // Parsing PID dan Name
            let head_parts: Vec<&str> = head.split_whitespace().collect();
            let len = head_parts.len();
            if len > 2 {
                initial_stat.pid = head_parts[0].parse().unwrap_or(0);
                initial_stat.comm = head_parts[1].to_string();
            }
            let parts: Vec<&str> = tail.split_whitespace().collect();
            let len = parts.len();
            if len > 0 {
                initial_stat.state = parts[0].to_string();
            }
            if len > 12 {
                initial_stat.utime = parts[11].parse::<u64>().unwrap_or(0);
                initial_stat.stime = parts[12].parse::<u64>().unwrap_or(0);
            }
            if len > 21 {
                initial_stat.vsize = parts[20].parse::<usize>().unwrap_or(0);
                initial_stat.rss = parts[21].parse::<usize>().unwrap_or(0) * page_size;
            }
        }
    }
    Mutex::new((initial_stat, Instant::now(), AtomicBool::new(true)))
});

static PROC_PROCESSING: LazyLock<AtomicBool> = LazyLock::new(|| AtomicBool::new(false));
const DURATION_CHECK: Duration = Duration::from_millis(500);

impl Stat {
    pub fn get() -> Self {
        Self::with_duration(DURATION_CHECK)
    }

    pub fn force_refresh() -> Self {
        Self::__reparse(true)
    }

    pub fn with_duration(duration: Duration) -> Self {
        {
            let lock = PROC_STATE_CACHE.lock();
            if lock.1.elapsed() < duration || lock.2.load(Ordering::Relaxed) == false {
                return lock.0.clone();
            }
        }
        Self::__reparse(false)
    }

    fn __reparse(force: bool) -> Self {
        if !force && PROC_PROCESSING.swap(true, Ordering::Acquire) {
            return PROC_STATE_CACHE.lock().0.clone();
        }

        let mut next_stat = Stat::default();
        let now = Instant::now();
        let page_size = unsafe { libc::sysconf(libc::_SC_PAGESIZE) as usize };

        if let Ok(content) = std::fs::read_to_string("/proc/self/stat") {
            if let Some(last_paren) = content.rfind(')') {
                let (head, tail) = content.split_at(last_paren + 1);

                // Parsing PID dan Name
                let head_parts: Vec<&str> = head.split_whitespace().collect();
                next_stat.pid = head_parts[0].parse().unwrap_or(0);
                next_stat.comm = head_parts[1].to_string();
                let parts: Vec<&str> = tail.split_whitespace().collect();
                let len = parts.len();
                if len > 0 {
                    next_stat.state = parts[0].to_string();
                }
                if len > 12 {
                    next_stat.utime = parts[11].parse::<u64>().unwrap_or(0);
                    next_stat.stime = parts[12].parse::<u64>().unwrap_or(0);
                }
                if len > 21 {
                    next_stat.vsize = parts[20].parse::<usize>().unwrap_or(0);
                    next_stat.rss = parts[21].parse::<usize>().unwrap_or(0) * page_size;
                }
            }
        }

        {
            let mut lock = PROC_STATE_CACHE.lock();
            let prev = &lock.0;
            let elapsed = lock.1.elapsed().as_secs_f64();

            if elapsed > 0.0 && prev.utime + prev.stime > 0 {
                let total_ticks =
                    (next_stat.utime + next_stat.stime).saturating_sub(prev.utime + prev.stime);

                // USER_HZ = 100
                let cpu_points = (total_ticks as f64 / 100.0) / elapsed;
                next_stat.cpu_usage = (cpu_points * 100.0);
            }

            lock.0 = next_stat.clone();
            lock.1 = now;
            lock.2.store(true, Ordering::Release);
        }

        PROC_PROCESSING.store(false, Ordering::Release);
        next_stat
    }
}
