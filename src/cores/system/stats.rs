use crate::cores::proc::mounts::{Device, Mounts};
use parking_lot::Mutex;
use serde::{Deserialize, Serialize};
use std::sync::LazyLock;
use std::time::{Duration, Instant};
use sysinfo::{Disks, Networks, System};

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, Eq, Hash)]
pub struct Disk {
    pub name: String,
    pub io_read: usize,
    pub io_write: usize,
    pub total: usize,
    pub used: usize,
    pub free: usize,
    pub kind: String,
    pub device: Option<Device>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct StatsRecord {
    pub network_tx: usize,
    pub network_rx: usize,
    pub cpu_percentage: f64,
    pub free_memory: usize,
    pub available_memory: usize,
    pub total_memory: usize,
    pub used_memory: usize,
    pub disks: Vec<Disk>,
}

const DURATION_CHECK: Duration = Duration::from_secs(1);
const MIN_DURATION_CHECK: Duration = Duration::from_millis(50);
static STATES: LazyLock<
    Mutex<(
        System,
        Networks,
        Disks,
        Instant,
        usize,
        usize,
        StatsRecord,
        Instant,
    )>,
> = LazyLock::new(|| {
    let mut s = System::new_all();
    let mut n = Networks::new_with_refreshed_list();
    let mut disks = Disks::new_with_refreshed_list();
    s.refresh_cpu_all();
    n.refresh(true);
    disks.refresh(true);
    s.refresh_memory();

    let initial_stats = StatsRecord {
        network_tx: 0,
        network_rx: 0,
        cpu_percentage: 0.0,
        free_memory: s.free_memory() as usize,
        total_memory: s.total_memory() as usize,
        available_memory: s.available_memory() as usize,
        used_memory: s.used_memory() as usize,
        disks: Stats::__disk_lists(&disks),
    };
    Mutex::new((
        s,
        n,
        disks,
        Instant::now(),
        0,
        0,
        initial_stats,
        Instant::now(),
    ))
});
#[derive(Debug, Clone)]
pub struct Stats;

impl Stats {
    /// Generates a list of detailed disk information from the provided `Disks` object.
    ///
    /// This function iterates over all the disks in the provided `Disks` instance,
    /// retrieves their usage and metadata, and constructs a `Vec<Disk>` containing
    /// detailed information about each disk.
    ///
    /// # Parameters
    /// - `disks`: A reference to a `Disks` object, which holds information about the system's disks.
    ///
    /// # Returns
    /// A `Vec<Disk>` containing detailed information for each disk, including:
    /// - `name`: The disk's name or its filesystem name if the name is unavailable.
    /// - `total`: The total disk space in bytes.
    /// - `used`: The used disk space in bytes.
    /// - `free`: The available/free disk space in bytes.
    /// - `io_read`: The number of bytes read from the disk during I/O operations.
    /// - `io_write`: The number of bytes written to the disk during I/O operations.
    /// - `device`: Additional device information, if available.
    ///
    /// # Details
    /// - The disk name is determined by calling `d.name()` and falling back to the
    ///   filesystem name if the disk name is empty.
    /// - The total, used, and free space are calculated from the disk's space properties.
    /// - Disk I/O statistics (read and write bytes) are fetched from the disk's usage data.
    /// - The `device` field is determined by looking up the disk name in the mount points
    ///   provided by `Storage::mount_points()`.
    ///
    /// # Example Usage
    /// ```rust
    /// let disks = Disks::new();
    /// let disk_list = __disk_lists(&disks);
    /// for disk in disk_list {
    ///     println!("Disk Name: {}", disk.name);
    ///     println!("Total Space: {} bytes", disk.total);
    ///     println!("Used Space: {} bytes", disk.used);
    ///     println!("Free Space: {} bytes", disk.free);
    ///     println!("I/O Read Bytes: {}", disk.io_read);
    ///     println!("I/O Write Bytes: {}", disk.io_write);
    /// }
    /// ```
    fn __disk_lists(disks: &Disks) -> Vec<Disk> {
        let mut disk_list = Vec::new();
        let storage_disk = Mounts::mount_points();
        for d in disks.list() {
            let usage = d.usage();
            let disk_name = if !d.name().to_string_lossy().is_empty() {
                d.name().to_string_lossy().into_owned()
            } else {
                format!("{}", d.file_system().to_string_lossy())
            };
            let device = storage_disk.get(&disk_name);
            disk_list.push(Disk {
                name: disk_name,
                total: d.total_space() as usize,
                used: (d.total_space() - d.available_space()) as usize,
                free: d.available_space() as usize,
                io_read: usage.read_bytes as usize,
                kind: d.kind().to_string(),
                io_write: usage.written_bytes as usize,
                device,
            });
        }
        disk_list
    }

    /// Collects and computes system statistics, including network and disk usage, as well as CPU percentage.
    ///
    /// This function locks the shared `STATES` structure to access and update system metrics. It performs the following operations:
    /// 1. If the elapsed time since the last check exceeds `DURATION_NETWORK`, it refreshes network statistics:
    ///    - Loops through available network interfaces, excluding the loopback interface (`"lo"`), to calculate total received (`network_rx`) and transmitted (`network_tx`) data.
    ///    - Updates the cached network statistics in `last_res`.
    /// 2. If the elapsed time since the last check exceeds `DURATION_HARDWARE`, it further refreshes hardware statistics:
    ///    - Updates CPU usage by refreshing the system's CPU data.
    ///    - Updates disk statistics by gathering information about the available disks.
    ///    - Stores CPU usage as a percentage (`cpu_percentage`) and the detailed disk data using a helper function `Self::__disk_lists`.
    ///    - Updates the `last_check` timestamp to the current time.
    ///
    /// Finally, the function returns a clone of the updated `StatsRecord` containing the newly computed metrics.
    ///
    /// # Returns
    /// A `StatsRecord` representing the latest system statistics.
    ///
    /// # Notes
    /// - This function makes heavy use of locking to ensure thread-safe access to shared resources.
    /// - Network updates occur more frequently, based on `DURATION_NETWORK`, while CPU and disk data are updated less often, depending on `DURATION_HARDWARE`.
    ///
    /// # Example
    /// ```
    /// let stats = statistic();
    /// println!("CPU Usage: {}%", stats.cpu_percentage);
    /// println!("Network RX: {} bytes, TX: {} bytes", stats.network_rx, stats.network_tx);
    /// ```
    pub fn statistic() -> StatsRecord {
        let mut lock = STATES.lock();
        let (sys, networks, disks, last_check, _acc_rx, _acc_tx, last_res, last_mem) = &mut *lock;
        if last_check.elapsed() >= DURATION_CHECK {
            let now = Instant::now();
            networks.refresh(true);
            let mut current_rx = 0;
            let mut current_tx = 0;
            for (name, data) in networks.list() {
                if name == "lo" {
                    continue;
                }
                current_rx += data.received() as usize;
                current_tx += data.transmitted() as usize;
            }
            last_res.network_rx = current_rx;
            last_res.network_tx = current_tx;
            sys.refresh_memory();
            sys.refresh_cpu_all();
            disks.refresh(true);

            last_res.cpu_percentage = sys.global_cpu_usage() as f64;
            last_res.disks = Self::__disk_lists(&disks);
            last_res.free_memory = sys.free_memory() as usize;
            last_res.available_memory = sys.available_memory() as usize;
            last_res.total_memory = sys.total_memory() as usize;
            last_res.used_memory = sys.used_memory() as usize;
            *last_check = now;
            *last_mem = now;
        }
        last_res.clone()
    }

    pub fn used_memory() -> usize {
        let (used_memory, ..) = Self::memory_statistic(Duration::from_secs(1));
        used_memory
    }
    pub fn free_memory() -> usize {
        let (_, free_memory, ..) = Self::memory_statistic(Duration::from_secs(1));
        free_memory
    }
    pub fn available_memory() -> usize {
        let (_, _, available_memory, _) = Self::memory_statistic(Duration::from_secs(1));
        available_memory
    }

    pub fn total_memory() -> usize {
        let (_, _, _, total_memory) = Self::memory_statistic(Duration::from_secs(1));
        total_memory
    }

    pub fn memory_statistic(duration: Duration) -> (usize, usize, usize, usize) {
        let mut lock = STATES.lock();
        let (sys, _, _, _, _, _, last_res, last_check_mem) = &mut *lock;
        let effective_duration = duration.max(MIN_DURATION_CHECK);

        if last_check_mem.elapsed() >= effective_duration {
            sys.refresh_memory();
            last_res.free_memory = sys.free_memory() as usize;
            last_res.available_memory = sys.available_memory() as usize;
            last_res.total_memory = sys.total_memory() as usize;
            last_res.used_memory = sys.used_memory() as usize;
            *last_check_mem = Instant::now();
        }
        (
            last_res.used_memory,
            last_res.free_memory,
            last_res.available_memory,
            last_res.total_memory,
        )
    }
}
