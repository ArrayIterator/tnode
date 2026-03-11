use crate::cores::generator::uuid::Uuid;
use crate::cores::helper::file_info::FileInfo;
use crate::cores::helper::user::{User, UserDetail};
use crate::cores::runner::console::ConsoleArguments;
use crate::cores::system::error::{Error, ResultError};
use const_format::concatcp;
use nix::libc;
use path_clean::PathClean;
use std::clone::Clone;
use std::env;
use std::os::linux::net::SocketAddrExt;
use std::os::unix::net::SocketAddr;
use std::path::{Path, PathBuf};
use std::sync::{LazyLock, OnceLock};

pub const APP_ENV_PREFIX: &str = "TN_";
pub const APP_NAME: &str = env!("CARGO_PKG_NAME");
pub const APP_VERSION: &str = env!("CARGO_PKG_VERSION");
pub const APP_DESCRIPTION: &str = env!("CARGO_PKG_DESCRIPTION");

pub const APP_AUTHOR: &str = env!("CARGO_PKG_AUTHORS");
pub const APP_BUILD_TIMESTAMP: &str = env!("BUILD_TIMESTAMP");
pub const APP_BUILD_TIMESTAMP_DATE: &str = env!("BUILD_TIMESTAMP_DATE");
pub const APP_VERSION_FULL: &str =
    concatcp!(APP_VERSION, " (built ", APP_BUILD_TIMESTAMP_DATE, ")");

pub const UPLOADS_BASE_NAME: &str = "uploads";
pub const DATAGRAM_BASE_NAME_PREFIX: &str = concatcp!(APP_NAME, "/dgram/", APP_VERSION);

pub static INITIAL_ROOT_USER: LazyLock<bool> = LazyLock::new(|| nix::unistd::geteuid().is_root());
pub static CURRENT_PID: LazyLock<u32> = LazyLock::new(|| std::process::id());
pub static EXE_FILE: LazyLock<PathBuf> =
    LazyLock::new(|| env::current_exe().unwrap_or(PathBuf::from(".")));

pub static CURRENT_EXE_DIRECTORY: LazyLock<PathBuf> = LazyLock::new(|| {
    (*EXE_FILE)
        .parent()
        .expect("Can not get parent directory")
        .to_path_buf()
});
pub static CURRENT_WORKING_DIRECTORY: LazyLock<PathBuf> =
    LazyLock::new(|| env::current_dir().unwrap_or_else(|_| PathBuf::from(".")));
pub static ROOT_DIR: LazyLock<PathBuf> = LazyLock::new(|| {
    let key = concatcp!(APP_ENV_PREFIX, "ROOT_DIR");
    let path = match env::var(key) {
        Ok(val) => {
            let p = PathBuf::from(val);
            if p.is_absolute() {
                p
            } else {
                CURRENT_WORKING_DIRECTORY.join(p)
            }
        }
        Err(_) => CURRENT_EXE_DIRECTORY.to_path_buf(),
    };
    path.clean()
});

pub static HAS_LIMIT_LESS: LazyLock<bool> =
    LazyLock::new(|| match env::var(concatcp!(APP_ENV_PREFIX, "NO_LIMIT")) {
        Ok(val) => {
            let val = val.trim();
            val.to_lowercase() == "true" || val == "1"
        }
        Err(_) => false,
    });

pub static U16_LIMITLESS: LazyLock<usize> = LazyLock::new(|| {
    let has_limit_less = *HAS_LIMIT_LESS;
    if has_limit_less { 1048576 } else { 65535 }
});

pub static VAR_DIRECTORY: LazyLock<PathBuf> = LazyLock::new(|| (*ROOT_DIR).join("var"));
pub static LOG_DIRECTORY: LazyLock<PathBuf> = LazyLock::new(|| (*VAR_DIRECTORY).join("log"));
pub static TEMP_DIRECTORY: LazyLock<PathBuf> = LazyLock::new(|| (*VAR_DIRECTORY).join("temp"));
pub static CACHE_DIRECTORY: LazyLock<PathBuf> = LazyLock::new(|| (*VAR_DIRECTORY).join("cache"));
pub static DOWNLOAD_CACHE_DIRECTORY: LazyLock<PathBuf> = LazyLock::new(|| (*CACHE_DIRECTORY).join("downloads"));
pub static LIB_DIRECTORY: LazyLock<PathBuf> = LazyLock::new(|| (*VAR_DIRECTORY).join("lib"));
pub static ACME_DIRECTORY: LazyLock<PathBuf> = LazyLock::new(|| (*LIB_DIRECTORY).join("acme"));
pub static STORAGE_DIRECTORY: LazyLock<PathBuf> = LazyLock::new(|| (*ROOT_DIR).join("storage"));
pub static PUBLIC_DIRECTORY: LazyLock<PathBuf> = LazyLock::new(|| (*ROOT_DIR).join("public"));
pub static UPLOADS_DIRECTORY: LazyLock<PathBuf> =
    LazyLock::new(|| (*STORAGE_DIRECTORY).join(UPLOADS_BASE_NAME));
pub static DATA_DIRECTORY: LazyLock<PathBuf> = LazyLock::new(|| (*STORAGE_DIRECTORY).join("data"));

pub static THEMES_DIRECTORY: LazyLock<PathBuf> = LazyLock::new(|| {
    let themes_dir = env::var(concatcp!(APP_ENV_PREFIX, "THEMES_DIR"))
        .unwrap_or_else(|_| (*ROOT_DIR).join("themes").to_string_lossy().to_string());
    if themes_dir.is_empty() {
        return (*ROOT_DIR).join("themes");
    }
    let mut path = PathBuf::from(themes_dir);
    if !path.is_absolute() {
        path = (*CURRENT_WORKING_DIRECTORY).join(path).to_path_buf();
    }
    path.clean().to_path_buf()
});

pub static MODULES_DIRECTORY: LazyLock<PathBuf> = LazyLock::new(|| (*ROOT_DIR).join("modules"));

pub static SHM_DIRECTORY: LazyLock<PathBuf> = LazyLock::new(|| PathBuf::from("/dev/shm"));
pub static SOCKET_DIRECTORY: LazyLock<PathBuf> = LazyLock::new(|| SHM_DIRECTORY.clone());
pub static CONFIG_FILE_NAME: LazyLock<PathBuf> = LazyLock::new(|| {
    // (*ROOT_DIR).join("config.yaml")
    let config_file = env::var(concatcp!(APP_ENV_PREFIX, "CONFIG_FILE")).unwrap_or_else(|_| {
        (*ROOT_DIR)
            .join("config.yaml")
            .to_string_lossy()
            .to_string()
    });
    if config_file.is_empty()
        || ![".yaml", ".yml"]
            .iter()
            .any(|ext| config_file.ends_with(ext))
    {
        return (*ROOT_DIR).join("config.yaml");
    }
    let mut path = PathBuf::from(config_file);
    if !path.is_absolute() {
        path = (*CURRENT_WORKING_DIRECTORY).join(path).to_path_buf();
    }
    path.clean().to_path_buf()
});

pub static MAYBE_VIA_SYSTEM_D: LazyLock<bool> = LazyLock::new(|| env::var("INVOCATION_ID").is_ok());
pub static MAYBE_VIA_JOURNAL: LazyLock<bool> = LazyLock::new(|| env::var("JOURNAL_STREAM").is_ok());

pub static BINARY_OWNER: LazyLock<Option<UserDetail>> = LazyLock::new(|| {
    let exe_file = EXE_FILE.to_string_lossy().to_string();
    FileInfo::new(exe_file).owner()
});

pub static CURRENT_USER: LazyLock<User> = LazyLock::new(|| User::current());
// check if running underdevelopment cargo
pub static IS_CARGO: LazyLock<bool> = LazyLock::new(|| {
    if cfg!(debug_assertions) {
        return true;
    }
    unsafe {
        let ppid = libc::getppid();
        let path = format!("/proc/{}/comm", ppid);
        if let Ok(name) = std::fs::read_to_string(path) {
            if name.trim() == "cargo" {
                return true;
            }
        }
    }
    env::var("CARGO").is_ok() || env::var("CARGO_EXE").is_ok()
});
static IS_DAEMON_CACHE: OnceLock<bool> = OnceLock::new();

pub struct Runtime;

impl Runtime {
    pub fn app_env_prefix() -> &'static str {
        APP_ENV_PREFIX
    }
    pub fn app_name() -> &'static str {
        APP_NAME
    }
    pub fn app_version() -> &'static str {
        APP_VERSION
    }
    pub fn app_author() -> &'static str {
        APP_AUTHOR
    }
    pub fn app_version_full() -> &'static str {
        APP_VERSION_FULL
    }
    pub fn app_description() -> &'static str {
        APP_DESCRIPTION
    }
    pub fn app_build_timestamp() -> &'static str {
        APP_BUILD_TIMESTAMP
    }
    pub fn app_build_timestamp_date() -> &'static str {
        APP_BUILD_TIMESTAMP_DATE
    }
    pub fn is_initial_root() -> bool {
        *INITIAL_ROOT_USER
    }
    pub fn pid() -> u32 {
        *CURRENT_PID
    }
    pub fn is_root() -> bool {
        Self::user().is_root()
    }
    pub fn user() -> &'static User {
        &*CURRENT_USER
    }
    pub fn exe_owner() -> &'static Option<UserDetail> {
        &*BINARY_OWNER
    }
    pub fn acme_dir() -> &'static Path {
        &*ACME_DIRECTORY
    }
    pub fn is_cargo_run() -> bool {
        *IS_CARGO
    }
    pub fn exe_file() -> &'static Path {
        &*EXE_FILE
    }
    pub fn exe_dir() -> &'static Path {
        &*CURRENT_EXE_DIRECTORY
    }
    pub fn root_dir() -> &'static Path {
        &*ROOT_DIR
    }
    pub fn cwd() -> &'static Path {
        &*CURRENT_WORKING_DIRECTORY
    }
    pub fn var_dir() -> &'static Path {
        &*VAR_DIRECTORY
    }
    pub fn log_dir() -> &'static Path {
        &*LOG_DIRECTORY
    }
    pub fn public_dir() -> &'static Path {
        &*PUBLIC_DIRECTORY
    }
    pub fn temp_dir() -> &'static Path {
        &*TEMP_DIRECTORY
    }
    pub fn temp_uploads_dir() -> PathBuf {
        (*TEMP_DIRECTORY).join(UPLOADS_BASE_NAME)
    }
    pub fn cahe_downloads_dir() -> &'static Path {
        &*DOWNLOAD_CACHE_DIRECTORY
    }
    pub fn cache_dir() -> &'static Path {
        &*CACHE_DIRECTORY
    }
    pub fn lib_dir() -> &'static Path {
        &*LIB_DIRECTORY
    }
    pub fn storage_dir() -> &'static Path {
        &*STORAGE_DIRECTORY
    }
    pub fn uploads_dir() -> &'static Path {
        &*UPLOADS_DIRECTORY
    }
    pub fn data_dir() -> &'static Path {
        &*DATA_DIRECTORY
    }
    pub fn themes_dir() -> &'static Path {
        &*THEMES_DIRECTORY
    }
    pub fn modules_dir() -> &'static Path {
        &*MODULES_DIRECTORY
    }
    pub fn shm_dir() -> &'static Path {
        &*SHM_DIRECTORY
    }
    pub fn socket_dir() -> &'static Path {
        &*SOCKET_DIRECTORY
    }
    pub fn config_file() -> &'static Path {
        &*CONFIG_FILE_NAME
    }
    pub fn socket_file() -> PathBuf {
        let current_uid = nix::unistd::getuid().as_raw() as u32;
        let socket_base_name = format!("{}-{}.sock", current_uid, APP_NAME);
        SOCKET_DIRECTORY.join(socket_base_name)
    }
    pub fn datagram_base_name() -> String {
        let current_uid = nix::unistd::getuid().as_raw() as u32;
        format!("{}/{}", DATAGRAM_BASE_NAME_PREFIX, current_uid)
    }
    pub fn current_dir() -> PathBuf {
        env::current_dir().unwrap()
    }

    pub fn datagram_client_identity() -> String {
        let identity = Self::datagram_base_name();
        format!("{}/client/{}", identity, Self::pid())
    }
    pub fn datagram_server_identity() -> String {
        let identity = Self::datagram_base_name();
        format!("{}/server", identity)
    }
    pub fn datagram_server_address() -> ResultError<SocketAddr> {
        let socket_control = Self::datagram_server_identity();
        SocketAddr::from_abstract_name(socket_control.as_bytes()).map_err(|e| {
            Error::address_not_available(format!(
                "Invalid socket server control @{}: {}",
                socket_control, e
            ))
        })
    }

    pub fn datagram_client_address() -> ResultError<SocketAddr> {
        let socket_control = Self::datagram_client_identity();
        SocketAddr::from_abstract_name(socket_control.as_bytes()).map_err(|e| {
            Error::address_not_available(format!(
                "Invalid socket client control @{}: {}",
                socket_control, e
            ))
        })
    }

    pub fn datagram_client_unique_address() -> ResultError<SocketAddr> {
        let uuid_v7 = Uuid::v7();
        let socket_control = format!("{}/{}", Self::datagram_client_identity(), uuid_v7);
        SocketAddr::from_abstract_name(&socket_control.as_bytes()).map_err(|e| {
            Error::address_not_available(format!(
                "Invalid socket client control @{}: {}",
                socket_control, e
            ))
        })
    }
    pub fn maybe_via_system_d() -> bool {
        *MAYBE_VIA_SYSTEM_D
    }
    pub fn maybe_via_journal() -> bool {
        *MAYBE_VIA_JOURNAL
    }
    pub fn config_file_of(
        global_arg: ConsoleArguments,
        local_arg: Option<ConsoleArguments>,
    ) -> PathBuf {
        if let Some(cfg) = &local_arg {
            if let Some(cfg_file) = &cfg.config {
                return cfg_file.clone();
            }
        }
        if let Some(cfg) = &global_arg.config {
            cfg.clone()
        } else {
            Self::config_file().to_path_buf()
        }
    }
    pub fn has_limit_less() -> bool {
        *HAS_LIMIT_LESS
    }
    pub fn u16_limitless() -> usize {
        *U16_LIMITLESS
    }

    pub fn is_daemon() -> bool {
        *IS_DAEMON_CACHE.get_or_init(|| unsafe {
            let is_atty = libc::isatty(0) != 0 || libc::isatty(1) != 0 || libc::isatty(2) != 0;
            let pgrep = libc::tcgetpgrp(0);
            let ppid = libc::getppid();
            !is_atty || pgrep == -1 || ppid == 1
        })
    }
}
