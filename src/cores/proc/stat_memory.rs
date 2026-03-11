use nix::libc;
use parking_lot::Mutex;
use serde::{Deserialize, Serialize};
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::LazyLock;
use std::time::{Duration, Instant};

static PROC_STATE_CACHE: LazyLock<Mutex<(StatMemory, Instant, AtomicBool)>> = LazyLock::new(|| {
    Mutex::new((
        StatMemory::default(),
        Instant::now(),
        AtomicBool::new(true), // boolean identity as initial value
    ))
});

const DURATION_CHECK: Duration = Duration::from_millis(500);
static PROC_PROCESSING: LazyLock<AtomicBool> = LazyLock::new(|| AtomicBool::new(false));

static PROC_LAST_FORCE_INSTANT: LazyLock<Mutex<Instant>> =
    LazyLock::new(|| Mutex::new(Instant::now()));
const DURATION_MIN_FORCE_IN_MS: u128 = 10;
#[derive(Debug, Default, Serialize, Deserialize, Clone, PartialEq, Eq, Hash)]
pub struct StatMemory {
    pub page_size: usize,
    pub size: usize,   // Total virtual memory (Vmsize)
    pub rss: usize,    // Resident set size
    pub shared: usize, // Shared pages (dari shared libs, dll)
    pub text: usize,   // Code segment
    pub lib: usize,    // Library
    pub data: usize,   // Data + Stack
    pub dirty: usize,  // Dirty pages
}

impl StatMemory {
    pub fn refresh() {
        Self::__force_reparse();
    }

    fn __force_reparse() -> Self {
        if PROC_PROCESSING.swap(true, Ordering::Acquire) {
            {
                let mut lock = PROC_STATE_CACHE.lock();
                return lock.0.clone();
            }
        }
        let instant = Instant::now();
        {
            let mut last = PROC_LAST_FORCE_INSTANT.lock();
            if last.elapsed().as_millis() < DURATION_MIN_FORCE_IN_MS {
                let lock = PROC_STATE_CACHE.lock();
                if !lock.2.load(Ordering::Acquire) {
                    PROC_PROCESSING.store(false, Ordering::Release);
                    return lock.0.clone();
                }
            }
            *last = instant;
        }
        let page_size = unsafe { libc::sysconf(libc::_SC_PAGESIZE) as usize };
        let mut sm = StatMemory {
            page_size,
            ..Default::default()
        };

        if let Ok(content) = std::fs::read_to_string("/proc/self/statm") {
            let parts: Vec<&str> = content.split_whitespace().collect();

            if parts.len() >= 7 {
                sm.size = parts[0].parse::<usize>().unwrap_or(0) * page_size;
                sm.rss = parts[1].parse::<usize>().unwrap_or(0) * page_size;
                sm.shared = parts[2].parse::<usize>().unwrap_or(0) * page_size;
                sm.text = parts[3].parse::<usize>().unwrap_or(0) * page_size;
                sm.lib = parts[4].parse::<usize>().unwrap_or(0) * page_size;
                sm.data = parts[5].parse::<usize>().unwrap_or(0) * page_size;
                sm.dirty = parts[6].parse::<usize>().unwrap_or(0) * page_size;
            }
            {
                let mut lock = PROC_STATE_CACHE.lock();
                lock.0 = sm.clone();
                lock.1 = instant;
                lock.2.store(true, Ordering::Release);
            }
        } else {
            {
                let lock = PROC_STATE_CACHE.lock();
                sm = lock.0.clone();
            }
        }
        PROC_PROCESSING.store(false, Ordering::Release);
        sm
    }

    pub fn get() -> Self {
        Self::with_duration(DURATION_CHECK)
    }

    pub fn with_duration(duration: Duration) -> Self {
        {
            let lock = PROC_STATE_CACHE.lock();
            if !lock.2.load(Ordering::Acquire) && lock.1.elapsed() < duration {
                return lock.0.clone();
            }
        }
        Self::__force_reparse()
    }

    pub fn private_rss(&self) -> usize {
        self.rss.saturating_sub(self.shared)
    }

    pub fn rss_mb(&self) -> f64 {
        self.rss as f64 / 1024.0 / 1024.0
    }
}
