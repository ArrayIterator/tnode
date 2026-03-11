use crate::cores::helper::hack::Hack;
use parking_lot::Mutex;
use serde::{Deserialize, Serialize};
use std::io::BufRead;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::LazyLock;
use std::time::{Duration, Instant};

static PROC_STATUS_CACHE: LazyLock<Mutex<(Status, Instant, AtomicBool)>> = LazyLock::new(|| {
    Mutex::new((
        Status::default(),
        Instant::now(),
        AtomicBool::new(true), // boolean identity as initial value
    ))
});

static PROC_LAST_FORCE_INSTANT: LazyLock<Mutex<Instant>> =
    LazyLock::new(|| Mutex::new(Instant::now()));
static PROC_PROCESSING: LazyLock<AtomicBool> = LazyLock::new(|| AtomicBool::new(false));
const DURATION_IN_SECONDS: Duration = Duration::from_secs(1);
const DURATION_MIN_FORCE_IN_MS: u128 = 10;

#[derive(Debug, Clone, Default, PartialEq, Eq, Hash, Deserialize, Serialize)]
pub struct SignalCapabilitiesStatus {
    pub sigq: String,   // signal queue
    pub sigpnd: String, // pending signals
    pub shdpnd: String, // shared pending signals
    pub sigblk: String, // blocked signals
    pub sigign: String, // ignored signals
    pub sigcgt: String, // caught signals
    pub capinh: String, // capabilities inherent
    pub capprm: String, // capabilities permitted
    pub capeff: String, // capabilities effective
    pub capbnd: String, // capabilities bounding
    pub capamb: String, // capabilities ambient
}

#[derive(Debug, Clone, Default, PartialEq, Eq, Hash, Deserialize, Serialize)]
pub struct IdentityStatus {
    pub name: String,           // name of the process
    pub umask: String,          // current umask
    pub state: String,          // state of the process R (Running)
    pub thread_group_id: usize, // thread group id
    pub num_group_id: usize,    // numerical group id
    pub pid: usize,             // process id
    pub ppid: usize,            // parent process id
    pub tracer_pid: usize,      // tracer pid
}

#[derive(Debug, Clone, Default, PartialEq, Eq, Hash, Deserialize, Serialize)]
pub struct CredentialStatus {
    pub uid: usize,         // real user id
    pub gid: usize,         // real group id
    pub euid: usize,        // effective user id
    pub egid: usize,        // effective group id
    pub suid: usize,        // saved set user id
    pub sgid: usize,        // saved set group id
    pub fsuid: usize,       // file system uid
    pub fsgid: usize,       // file system gid
    pub fd_size: usize,     // file descriptor size
    pub groups: Vec<usize>, // supplementary group list
}

#[derive(Debug, Clone, Default, PartialEq, Eq, Hash, Deserialize, Serialize)]
pub struct NameSpaceThreadStatus {
    // NStgid:	1807971
    // NSpid:	1807971
    // NSpgid:	1807971
    // NSsid:	1795943
    // Kthread:	1795943
    pub thread_group_id: usize, // thread group id
    pub pid: usize,             // process id
    pub ppid: usize,            // parent process id
    pub sid: usize,             // session id
    pub kthread: usize,         // kernel thread total
}

#[derive(Debug, Clone, Default, PartialEq, Eq, Hash, Deserialize, Serialize)]
pub struct MemoryStatus {
    pub vm_hwm: usize,     // High water mark (peak RSS)
    pub vm_peak: usize,    // Peak virtual memory size
    pub vm_rss: usize,     // Resident Set Size (current)
    pub vm_size: usize,    // Virtual memory size
    pub vm_pin: usize,     // Virtual memory size
    pub vm_data: usize,    // Data segment size
    pub vm_stk: usize,     // Stack segment size
    pub vm_exe: usize,     // Executable segment size
    pub vm_lib: usize,     // Library segment size
    pub vm_pte: usize,     // Page table entries size
    pub vm_lck: usize,     // Locked memory size
    pub vm_swap: usize,    // Swap space size
    pub rss_anon: usize,   // Anonymous RSS
    pub rss_file: usize,   // File-backed RSS
    pub rss_sh_mem: usize, // Shared memory size
}

impl CredentialStatus {
    fn split_id<T: AsRef<str>>(&mut self, value: T) -> (usize, usize, usize, usize) {
        let mut split = value
            .as_ref()
            .split_whitespace()
            .collect::<Vec<&str>>()
            .iter()
            .filter(|s| s.len() > 0)
            .map(|s| s.parse::<usize>().unwrap_or(0))
            .collect::<Vec<usize>>();
        let id = split.pop().unwrap_or(0);
        let e_id = split.pop().unwrap_or(id);
        let s_id = split.pop().unwrap_or(id);
        let fs_id = split.pop().unwrap_or(id);
        (id, e_id, s_id, fs_id)
    }
    fn set_data<K: AsRef<str>, V: AsRef<str>>(&mut self, key: K, value: V) -> bool {
        let value = value.as_ref();
        match key.as_ref() {
            "fdsize" => self.fd_size = value.parse::<usize>().unwrap_or(0),
            "groups" => {
                for gid in value.split_whitespace() {
                    let gid = gid.parse::<usize>().unwrap_or(0);
                    if gid == 0 {
                        continue;
                    }
                    self.groups.push(gid);
                }
            }
            "uid" => {
                let (uid, e_uid, s_uid, fs_uid) = self.split_id(value);
                self.uid = uid;
                self.euid = e_uid;
                self.suid = s_uid;
                self.fsuid = fs_uid;
            }
            "gid" => {
                let (gid, e_gid, s_gid, fs_gid) = self.split_id(value);
                self.gid = gid;
                self.egid = e_gid;
                self.sgid = s_gid;
                self.fsgid = fs_gid;
            }
            _ => return false,
        }
        true
    }
}
impl IdentityStatus {
    fn set_data<K: AsRef<str>, V: AsRef<str>>(&mut self, key: K, value: V) -> bool {
        match key.as_ref() {
            "name" => self.name = value.as_ref().to_string(),
            "state" => self.state = value.as_ref().to_string(),
            "umask" => self.umask = value.as_ref().to_string(),
            (e) => {
                let value = value.as_ref().parse::<usize>().unwrap_or(0);
                match e {
                    "tgid" => self.thread_group_id = value,
                    "pid" => self.pid = value,
                    "ppid" => self.ppid = value,
                    "tracerpid" => self.tracer_pid = value,
                    _ => return false,
                }
                return true;
            }
        }
        true
    }
}

impl NameSpaceThreadStatus {
    fn set_data<K: AsRef<str>, V: AsRef<str>>(&mut self, key: K, value: V) -> bool {
        let value = value.as_ref();
        match key.as_ref() {
            "nstgid" => self.thread_group_id = value.parse::<usize>().unwrap_or(0),
            "nspid" => self.pid = value.parse::<usize>().unwrap_or(0),
            "nspgid" => self.ppid = value.parse::<usize>().unwrap_or(0),
            "nssid" => self.sid = value.parse::<usize>().unwrap_or(0),
            "kthread" => self.kthread = value.parse::<usize>().unwrap_or(0),
            _ => return false,
        }
        true
    }
}
impl MemoryStatus {
    fn set_data<K: AsRef<str>, V: AsRef<str>>(&mut self, key: K, value: V) -> bool {
        match key.as_ref() {
            "vmpeak" => self.vm_peak = Hack::size_to_bytes(value).unwrap_or(0) as usize,
            "vmsize" => self.vm_size = Hack::size_to_bytes(value).unwrap_or(0) as usize,
            "vmlck" => self.vm_lck = Hack::size_to_bytes(value).unwrap_or(0) as usize,
            "vmpin" => self.vm_pin = Hack::size_to_bytes(value).unwrap_or(0) as usize,
            "vmhwm" => self.vm_hwm = Hack::size_to_bytes(value).unwrap_or(0) as usize,
            "vmrss" => self.vm_rss = Hack::size_to_bytes(value).unwrap_or(0) as usize,
            "rssanon" => self.rss_anon = Hack::size_to_bytes(value).unwrap_or(0) as usize,
            "rssfile" => self.rss_file = Hack::size_to_bytes(value).unwrap_or(0) as usize,
            "rssshmem" => self.rss_sh_mem = Hack::size_to_bytes(value).unwrap_or(0) as usize,
            "vmdata" => self.vm_data = Hack::size_to_bytes(value).unwrap_or(0) as usize,
            "vmstk" => self.vm_stk = Hack::size_to_bytes(value).unwrap_or(0) as usize,
            "vmexe" => self.vm_exe = Hack::size_to_bytes(value).unwrap_or(0) as usize,
            "vmlib" => self.vm_lib = Hack::size_to_bytes(value).unwrap_or(0) as usize,
            "vmpte" => self.vm_pte = Hack::size_to_bytes(value).unwrap_or(0) as usize,
            "vmswap" => self.vm_swap = Hack::size_to_bytes(value).unwrap_or(0) as usize,
            _ => return false,
        }
        true
    }
}
impl SignalCapabilitiesStatus {
    fn set_data<K: AsRef<str>, V: AsRef<str>>(&mut self, key: K, value: V) -> bool {
        match key.as_ref() {
            "sigq" => self.sigq = value.as_ref().to_string(),
            "sigpnd" => self.sigpnd = value.as_ref().to_string(),
            "shdpnd" => self.shdpnd = value.as_ref().to_string(),
            "sigblk" => self.sigblk = value.as_ref().to_string(),
            "sigign" => self.sigign = value.as_ref().to_string(),
            "sigcgt" => self.sigcgt = value.as_ref().to_string(),
            "capinh" => self.capinh = value.as_ref().to_string(),
            "capprm" => self.capprm = value.as_ref().to_string(),
            "capeff" => self.capeff = value.as_ref().to_string(),
            "capbnd" => self.capbnd = value.as_ref().to_string(),
            "capamb" => self.capamb = value.as_ref().to_string(),
            _ => return false,
        }
        true
    }
}

#[derive(Debug, Clone, Default, PartialEq, Eq, Hash, Deserialize, Serialize)]
pub struct Status {
    pub credential: CredentialStatus,
    pub identity: IdentityStatus,
    pub memory: MemoryStatus,
    pub signal_capabilities: SignalCapabilitiesStatus,
    pub namespace_thread: NameSpaceThreadStatus,
}

impl Status {
    pub fn refresh() {
        Self::__force_reparse();
    }
    fn __force_reparse() -> Self {
        if PROC_PROCESSING.swap(true, Ordering::Acquire) {
            {
                let mut lock = PROC_STATUS_CACHE.lock();
                return lock.0.clone();
            }
        }
        let instant = Instant::now();
        {
            let mut last = PROC_LAST_FORCE_INSTANT.lock();
            if last.elapsed().as_millis() < DURATION_MIN_FORCE_IN_MS {
                let lock = PROC_STATUS_CACHE.lock();
                if !lock.2.load(Ordering::Acquire) {
                    PROC_PROCESSING.store(false, Ordering::Release);
                    return lock.0.clone();
                }
            }
            *last = instant;
        }
        let mut status = Status::default();
        if let Ok(file) = std::fs::File::open("/proc/self/status") {
            let reader = std::io::BufReader::new(file);
            for line in reader.lines().flatten() {
                if let Some((key, value)) = line
                    .split_once(':')
                    .map(|(k, v)| (k.trim().to_lowercase(), v.trim()))
                {
                    if status.identity.set_data(&key, value) {
                        continue;
                    }
                    if status.memory.set_data(&key, value) {
                        continue;
                    }
                    if status.credential.set_data(&key, value) {
                        continue;
                    }
                    if status.signal_capabilities.set_data(&key, value) {
                        continue;
                    }
                    if status.namespace_thread.set_data(&key, value) {
                        continue;
                    }
                }
            }
            {
                let mut lock = PROC_STATUS_CACHE.lock();
                let prev = lock.0.clone();
                lock.0 = status.clone();
                lock.1 = Instant::now();
                lock.2.store(false, Ordering::Release);
            }
        } else {
            {
                let lock = PROC_STATUS_CACHE.lock();
                if !lock.2.load(Ordering::Acquire) {
                    status = lock.0.clone();
                }
            }
        }
        PROC_PROCESSING.store(false, Ordering::Release);
        status
    }

    pub fn with_duration(duration: Duration) -> Self {
        {
            let lock = PROC_STATUS_CACHE.lock();
            if !lock.2.load(Ordering::Acquire) && lock.1.elapsed() < duration {
                return lock.0.clone();
            }
        }
        Self::__force_reparse()
    }

    pub fn get() -> Self {
        Self::with_duration(DURATION_IN_SECONDS)
    }
}
