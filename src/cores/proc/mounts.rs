use parking_lot::Mutex;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::io::BufRead;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::LazyLock;
use std::time::{Duration, Instant};

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, Eq, Hash)]
pub struct Device {
    pub label: Option<LabeledDevice>,
    pub device: String,
    pub mount_point: String,
    pub fs_type: String,
    pub flags: Vec<String>,
    pub dump: u32,
    pub pass: u32,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, Eq, Hash)]
pub struct LabeledDevice {
    pub mount_point: String,
    pub label: String,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct LabeledDevices {
    pub devices: HashMap<String, LabeledDevice>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Devices {
    pub devices: HashMap<String, Device>,
}

impl LabeledDevices {
    pub fn get(&self, mount_point: &str) -> Option<LabeledDevice> {
        self.devices.get(mount_point).cloned()
    }
}

impl Device {
    // logical
    pub const EXT4: &'static str = "ext4";
    pub const EXT3: &'static str = "ext3";
    pub const EXT2: &'static str = "ext2";
    pub const XFS: &'static str = "xfs";
    pub const BTRFS: &'static str = "btrfs";
    pub const NTFS: &'static str = "ntfs";
    pub const FAT32: &'static str = "vfat";
    pub const EXFAT: &'static str = "exfat";
    pub const F2FS: &'static str = "f2fs";
    pub const ZFS: &'static str = "zfs";
    pub const HFSPLUS: &'static str = "hfsplus";
    // virtual
    pub const TMPFS: &'static str = "tmpfs"; // RAM disk
    pub const SQUASHFS: &'static str = "squashfs"; // SNAP, read-only
    pub const DEVTMPFS: &'static str = "devtmpfs"; // Device nodes
    pub const PROC: &'static str = "proc"; // Kernel info
    pub const SYSFS: &'static str = "sysfs"; // Kernel info
    pub const OVERLAY: &'static str = "overlay"; // Docker/Container
    pub const NFS: &'static str = "nfs"; // Network storage
    pub const CIFS: &'static str = "cifs"; // Windows share (Samba)
    pub const ISO9660: &'static str = "iso9660"; // CD/DVD/ISO image
}

impl Device {
    const PHYSICAL_PREFIXES: &'static [&'static str] = &[
        "/dev/sd",
        "/dev/vd",
        "/dev/nvme",
        "/dev/mmcblk",
        "/dev/mapper/",
    ];
    pub fn get_device(&self) -> &str {
        &self.device
    }

    pub fn is_physical(&self) -> bool {
        Self::PHYSICAL_PREFIXES
            .iter()
            .any(|&i| self.device.starts_with(i))
    }
    pub fn is_readonly(&self) -> bool {
        self.flags.contains(&"ro".to_string())
    }
    pub fn is_virtual(&self) -> bool {
        !self.is_physical()
    }
    pub fn is_tmpfs(&self) -> bool {
        self.fs_type == Device::TMPFS
    }
    pub fn is_squashfs(&self) -> bool {
        self.fs_type == Device::SQUASHFS
    }
    pub fn is_overlay(&self) -> bool {
        self.fs_type == Device::OVERLAY
    }
    pub fn is_devtmpfs(&self) -> bool {
        self.fs_type == Device::DEVTMPFS
    }
    pub fn is_proc(&self) -> bool {
        self.fs_type == Device::PROC
    }
    pub fn is_sysfs(&self) -> bool {
        self.fs_type == Device::SYSFS
    }
    pub fn is_nfs(&self) -> bool {
        self.fs_type == Device::NFS
    }
    pub fn is_cifs(&self) -> bool {
        self.fs_type == Device::CIFS
    }
    pub fn is_iso9660(&self) -> bool {
        self.fs_type == Device::ISO9660
    }
    pub fn is_zfs(&self) -> bool {
        self.fs_type == Device::ZFS
    }
    pub fn is_hfsplus(&self) -> bool {
        self.fs_type == Device::HFSPLUS
    }
    pub fn is_ext4(&self) -> bool {
        self.fs_type == Device::EXT4
    }
    pub fn is_ext3(&self) -> bool {
        self.fs_type == Device::EXT3
    }
    pub fn is_ext2(&self) -> bool {
        self.fs_type == Device::EXT2
    }
    pub fn is_xfs(&self) -> bool {
        self.fs_type == Device::XFS
    }
    pub fn is_btrfs(&self) -> bool {
        self.fs_type == Device::BTRFS
    }
}

impl Devices {
    pub fn new(vec_points: Vec<Device>) -> Self {
        let hash_map = HashMap::from_iter(vec_points.iter().map(|m| (m.device.clone(), m.clone())));
        Devices { devices: hash_map }
    }
    pub fn get(&self, device: &str) -> Option<Device> {
        self.devices.get(device).cloned()
    }
}

impl Default for Devices {
    fn default() -> Self {
        Mounts::mount_points()
    }
}

static PROC_MOUNT_CACHE: LazyLock<Mutex<(Devices, Instant, AtomicBool)>> = LazyLock::new(|| {
    Mutex::new((
        Devices {
            devices: HashMap::new(),
        },
        Instant::now(),
        AtomicBool::new(true), // boolean identity as initial value
    ))
});

static PROC_LABEL_CACHE: LazyLock<Mutex<(LabeledDevices, Instant, AtomicBool)>> =
    LazyLock::new(|| {
        Mutex::new((
            LabeledDevices {
                devices: HashMap::new(),
            },
            Instant::now(),
            AtomicBool::new(true), // boolean identity as initial value
        ))
    });

// Duration to recheck
const DURATION_DISKS: Duration = Duration::from_secs(30);
const MIN_DURATION_DISKS: Duration = Duration::from_secs(1);
static PROCESSING_DISK_REFRESH: Mutex<AtomicBool> = Mutex::new(AtomicBool::new(false));
static PROCESSING_LABEL_REFRESH: Mutex<AtomicBool> = Mutex::new(AtomicBool::new(false));
static IS_REFRESHING_DISK: AtomicBool = AtomicBool::new(false);
static IS_REFRESHING_LABEL: AtomicBool = AtomicBool::new(false);

pub struct Mounts;

impl Mounts {
    /// Refreshes the current state by reinitializing or updating the mount points.
    ///
    /// This function is a wrapper that triggers the internal process to reset or update
    /// the mount points used by the program. It is likely called when changes to the
    /// system's mount configuration need to be reflected in the application's state.
    ///
    /// # Example
    /// ```rust
    /// MyStruct::refresh(); // Refreshes the mount points associated with MyStruct.
    /// ```
    ///
    /// # Notes
    /// - The actual implementation of `___mount_points` is considered internal and is not
    ///   exposed to the caller.
    /// - This function assumes the presence of an internal method called `___mount_points`
    ///   within the `Self` context (e.g., impl block).
    /// - Proper error handling (if any) should be part of the `___mount_points` implementation.
    ///
    /// # Safety
    /// Ensure that this function is called in a valid system state to avoid unexpected behavior
    /// during reinitialization.
    pub fn refresh() {
        Self::___mount_points();
    }

    //noinspection DuplicatedCode
    fn ___mount_points() -> Devices {
        if IS_REFRESHING_DISK.swap(true, Ordering::Acquire) {
            return PROC_MOUNT_CACHE.lock().0.clone();
        }
        {
            let lock = PROC_MOUNT_CACHE.lock();
            let allowed = !lock.2.load(Ordering::Acquire) && lock.1.elapsed() >= MIN_DURATION_DISKS;
            if allowed {
                return lock.0.clone();
            }
        }

        let labeled_devices = Self::labeled_devices();
        let mut mount_points = Vec::new();
        if let Ok(file) = std::fs::File::open("/proc/self/mounts") {
            let reader = std::io::BufReader::new(file);
            for line in reader.lines().flatten() {
                let parts: Vec<&str> = line.split_whitespace().collect();
                if parts.len() >= 6 {
                    let device = parts[0].to_string();
                    mount_points.push(Device {
                        label: labeled_devices.get(&device),
                        device,
                        mount_point: parts[1].to_string(),
                        fs_type: parts[2].to_lowercase(),
                        flags: parts[3].split(',').map(|s| s.to_string()).collect(),
                        dump: parts[4].parse::<u32>().unwrap_or(0),
                        pass: parts[5].parse::<u32>().unwrap_or(0),
                    });
                }
            }
        }

        let result = Devices::new(mount_points);
        {
            let mut lock = PROC_MOUNT_CACHE.lock();
            lock.0 = result.clone();
            lock.1 = Instant::now();
            lock.2.store(false, Ordering::Release);
        }

        IS_REFRESHING_DISK.store(false, Ordering::Release);
        result
    }

    //noinspection DuplicatedCode
    pub fn __labeled_devices() -> LabeledDevices {

        {
            let lock = PROC_LABEL_CACHE.lock();
            let allowed = !lock.2.load(Ordering::Acquire) && lock.1.elapsed() >= MIN_DURATION_DISKS;
            if allowed {
                return lock.0.clone();
            }
        }
        // Cek processing flag
        {
            let is_processing = PROCESSING_LABEL_REFRESH.lock();
            if is_processing.load(Ordering::Relaxed) {
                let lock = PROC_LABEL_CACHE.lock();
                return lock.0.clone();
            }
            is_processing.store(true, Ordering::Relaxed);
        }

        let mut devices_map = HashMap::new();
        if let Ok(entries) = std::fs::read_dir("/dev/disk/by-label") {
            for entry in entries.flatten() {
                let symlink_path = entry.path();
                if let Ok(actual_path) = std::fs::canonicalize(&symlink_path) {
                    let dev_path = actual_path.to_string_lossy().into_owned();
                    let label_name = entry.file_name().to_string_lossy().into_owned();
                    devices_map.insert(
                        dev_path.clone(),
                        LabeledDevice {
                            mount_point: dev_path,
                            label: label_name,
                        },
                    );
                }
            }
        }

        let result = LabeledDevices {
            devices: devices_map,
        };

        // Update Cache
        {
            let mut lock = PROC_LABEL_CACHE.lock();
            lock.0 = result.clone();
            lock.1 = Instant::now();
            lock.2.store(false, Ordering::Relaxed);
        }

        PROCESSING_LABEL_REFRESH
            .lock()
            .store(false, Ordering::Relaxed);
        result
    }

    fn __lock_fn<S: Clone>(
        lock:  &Mutex<(S, Instant, AtomicBool)>,
        handler: fn() -> S,
    ) -> S {
        {
            let lock = lock.lock();
            let is_initial = lock.2.load(Ordering::Relaxed);
            let expired = lock.1.elapsed() >= DURATION_DISKS;
            if !is_initial && !expired {
                return lock.0.clone();
            }
        }
        handler()
    }

    pub fn labeled_devices() -> LabeledDevices {
        Self::__lock_fn(&PROC_LABEL_CACHE, Self::__labeled_devices)
    }

    pub fn mount_points() -> Devices {
        Self::__lock_fn(&PROC_MOUNT_CACHE, Self::___mount_points)
    }
}
