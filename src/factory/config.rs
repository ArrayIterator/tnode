#![deny(unused_imports)]

use crate::cores::auth::csrf_duration::CsrfDuration;
use crate::cores::auth::session::SessionManager;
use crate::cores::auth::session_tokenizer::SessionTokenizer;
use crate::cores::database::configuration::Configuration as DatabaseConfig;
use crate::cores::generator::uuid::Uuid;
use crate::cores::helper::file_info::FileInfo;
use crate::cores::helper::hack::Hack;
use crate::cores::helper::user::User;
use crate::cores::libs::redis::RedisConfig;
use crate::cores::net::ip::Ip;
use crate::cores::runner::console::ConsoleArguments;
use crate::cores::system::error::{Error, ResultError};
use crate::cores::system::event_manager::EventManager;
use crate::cores::system::runtime::Runtime;
use crate::factory::constant::{
    DEFAULT_AUTO_CLEAN_MEMORY_INTERVAL, DEFAULT_AUTO_CLEAN_MEMORY_SIZE_BYTES, DEFAULT_BACKLOG,
    DEFAULT_CONFIG, DEFAULT_CONNECTIONS, DEFAULT_CSRF_DURATION_MINUTES, DEFAULT_DISCONNECT_TIMEOUT,
    DEFAULT_KEEP_ALIVE, DEFAULT_OPERATION_TIMEOUT, DEFAULT_REQUEST_TIMEOUT, DEFAULT_RLIMIT_NOFILE,
    DEFAULT_SESSION_NAME, DEFAULT_SSL_SESSION_CACHE, DEFAULT_TCP, MAX_AUTO_CLEAN_MEMORY_INTERVAL,
    MAX_BACKLOG, MAX_CONNECTION_RATE, MAX_CONNECTIONS, MAX_CSRF_DURATION_MINUTES,
    MAX_DISCONNECT_TIMEOUT, MAX_KEEP_ALIVE, MAX_OPERATION_TIMEOUT, MAX_REQUEST_TIMEOUT,
    MAX_RLIMIT_NOFILE, MAX_SSL_SESSION_CACHE, MIN_AUTO_CLEAN_MEMORY_INTERVAL,
    MIN_AUTO_CLEAN_MEMORY_SIZE_BYTES, MIN_BACKLOG, MIN_CONNECTION_RATE, MIN_CONNECTIONS,
    MIN_CSRF_DURATION_MINUTES, MIN_DISCONNECT_TIMEOUT, MIN_KEEP_ALIVE, MIN_OPERATION_TIMEOUT,
    MIN_REQUEST_TIMEOUT, MIN_RLIMIT_NOFILE, MIN_SSL_SESSION_CACHE, DEFAULT_DASHBOARD_PATH
};
use crate::factory::factory::Factory;
use actix_web::http::KeepAlive;
use colored::*;
use log::{debug, trace, warn};
use path_clean::PathClean;
use serde::{Deserialize, Serialize};
use std::collections::{BTreeMap, HashMap, HashSet};
use std::os::linux::net::SocketAddrExt;
use std::os::unix::net::SocketAddr;
use std::path::{Path, PathBuf};
use std::sync::{Arc, LazyLock, OnceLock};
use std::time::Duration;
use regex::Regex;
use crate::cores::database::connection::Connection;
use crate::factory::ssl_storage::SSLStorage;

static RE_DASHBOARD_PATH: LazyLock<Regex> = LazyLock::new(|| {
    Regex::new(r"^[a-z0-9]([a-z0-9_\-]*[a-z0-9])?$").unwrap()
});

static RE_DASHBOARD_PATH_UNWANTED_CHARS: LazyLock<Regex> = LazyLock::new(|| {
    Regex::new(r"[^a-z0-9_\-]").unwrap()
});

#[derive(Debug, Clone, Deserialize, Serialize,)]
pub struct SSLKeyCert {
    pub(crate) key: String,
    pub(crate) cert: String,
}

impl SSLKeyCert {
    pub fn key(&self) -> String {
        self.key.clone()
    }
    pub fn cert(&self) -> String {
        self.cert.clone()
    }
}

#[derive(Debug, Clone, Deserialize, Serialize,)]
pub struct SSLConfig {
    #[serde(default)]
    auto_create: bool,
    #[serde(default)]
    listen: Vec<String>,
    #[serde(default)]
    key: String,
    #[serde(default)]
    cert: String,
    #[serde(default)]
    session_cache: usize,
    #[serde(default)]
    domains: Option<HashMap<String, SSLKeyCert>>,
    #[serde(flatten)]
    ___flatten: BTreeMap<String, serde_yaml::Value>,
}

impl SSLConfig {
    pub fn listen(&self) -> Vec<String> {
        self.listen.clone()
    }
    pub fn key(&self) -> String {
        self.key.clone()
    }
    pub fn cert(&self) -> String {
        self.cert.clone()
    }
    pub fn session_cache(&self) -> usize {
        self.session_cache
    }
    pub fn is_ssl_configured(&self) -> bool {
        !self.listen.is_empty() && !self.key.is_empty() && !self.cert.is_empty()
    }
    pub fn domains(&self) -> Option<HashMap<String, SSLKeyCert>> {
        self.domains.clone()
    }
    pub fn auto_create(&self) -> bool {
        self.auto_create
    }
    pub fn flatten(&self) -> BTreeMap<String, serde_yaml::Value> {
        self.___flatten.clone()
    }
}

impl Default for SSLConfig {
    fn default() -> Self {
        Self {
            auto_create: true,
            listen: vec![],
            key: "".to_string(),
            cert: "".to_string(),
            session_cache: DEFAULT_SSL_SESSION_CACHE,
            domains: None,
            ___flatten: Default::default(),
        }
    }
}

#[derive(Debug, Clone, Deserialize, Serialize, Default)]
pub struct Licenses {
    #[serde(flatten)]
    __flatten: BTreeMap<String, serde_yaml::Value>,
}

impl Licenses {
    pub fn flatten(&self) -> BTreeMap<String, serde_yaml::Value> {
        self.__flatten.clone()
    }
    pub fn get(&self, key: &str) -> Option<&serde_yaml::Value> {
        self.__flatten.get(key)
    }
}

#[derive(Debug, Clone, Deserialize, Serialize, Default)]
pub struct AppConfig {
    #[serde(default = "socket_file")]
    socket: String,
    #[serde(default = "default_session_name")]
    session_name: String,
    #[serde(default = "default_csrf_durations")]
    csrf_durations: usize,
    #[serde(default = "default_user")]
    user: String,
    #[serde(default = "datagram_identity")]
    datagram: String,
    #[serde(default = "default_server_tcp_listener")]
    tcp: Vec<String>,
    #[serde(default)]
    ssl: SSLConfig,
    #[serde(default = "default_workers")]
    workers: usize,
    #[serde(default = "default_mode")]
    mode: String,
    #[serde(default="default_dashboard_path")]
    dashboard_path: String,
    #[serde(default = "default_operation_timeout")]
    operation_timeout: usize,
    #[serde(default = "default_request_timeout")]
    request_timeout: usize,
    #[serde(default = "default_disconnect_timeout")]
    disconnect_timeout: usize,
    #[serde(default = "default_secret_key")]
    secret_key: String,
    #[serde(default = "default_salt_key")]
    salt_key: String,
    #[serde(default = "default_max_connections")]
    max_connections: usize,
    #[serde(default = "default_max_connections_rate")]
    max_connections_rate: usize,
    #[serde(default = "default_backlog")]
    backlog: u32,
    #[serde(default = "default_rlimit_nofile")]
    rlimit_nofile: usize,
    #[serde(default = "default_keepalive")]
    keep_alive: usize,
    #[serde(default = "default_auto_clean_memory")]
    auto_clean_memory: bool,
    #[serde(default = "default_auto_clean_memory_size")]
    auto_clean_memory_size: String,
    #[serde(default = "default_auto_clean_memory_interval")]
    auto_clean_memory_interval: String,
    #[serde(flatten)]
    ___flatten: Arc<BTreeMap<String, serde_yaml::Value>>,
}
pub fn default_session_name() -> String {
    DEFAULT_SESSION_NAME.to_string()
}

pub fn default_user() -> String {
    let mut user = "".to_string();
    if Runtime::is_root() {
        if let Some(binary_owner) = Runtime::exe_owner() {
            user = binary_owner.user.name.clone()
        }
    }
    if user.is_empty() {
        user = Runtime::user().name.clone();
    }
    if User::from_name(&user).is_root() {
        user = "www-data".to_string();
    }
    user
}

pub fn default_auto_clean_memory_interval() -> String {
    DEFAULT_AUTO_CLEAN_MEMORY_INTERVAL.to_string()
}

pub fn default_auto_clean_memory_size() -> String {
    DEFAULT_AUTO_CLEAN_MEMORY_SIZE_BYTES.to_string()
}
pub fn default_dashboard_path() -> String {
    DEFAULT_DASHBOARD_PATH.to_string()
}
pub fn default_auto_clean_memory() -> bool {
    true
}
pub fn default_ssl_session_cache() -> usize {
    DEFAULT_SSL_SESSION_CACHE
}

pub fn default_keepalive() -> usize {
    DEFAULT_KEEP_ALIVE
}
pub fn default_csrf_durations() -> usize {
    DEFAULT_CSRF_DURATION_MINUTES
}
pub fn default_rlimit_nofile() -> usize {
    DEFAULT_RLIMIT_NOFILE as usize
}
pub fn default_salt_key() -> String {
    "".to_string()
}
pub fn default_secret_key() -> String {
    "".to_string()
}
pub fn default_mode() -> String {
    "production".to_string()
}
pub fn default_operation_timeout() -> usize {
    DEFAULT_OPERATION_TIMEOUT
}
pub fn default_request_timeout() -> usize {
    DEFAULT_REQUEST_TIMEOUT
}
pub fn default_disconnect_timeout() -> usize {
    DEFAULT_DISCONNECT_TIMEOUT
}
pub fn default_server_tcp_listener() -> Vec<String> {
    vec![DEFAULT_TCP.to_string()]
}

pub fn socket_file() -> String {
    Runtime::socket_file().to_string_lossy().to_string()
}

pub fn datagram_identity() -> String {
    Runtime::datagram_base_name()
}

pub fn default_max_connections() -> usize {
    DEFAULT_CONNECTIONS
}
pub fn default_workers() -> usize {
    std::thread::available_parallelism()
        .map(|n| n.get())
        .unwrap_or(1)
}
pub fn default_max_connections_rate() -> usize {
    default_workers() * 256
}
pub fn default_backlog() -> u32 {
    DEFAULT_BACKLOG
}

/// Configuration struct for the application, encompassing details for
/// database and server setup.
///
/// This struct is used to deserialize configuration data, typically
/// from an external source such as a configuration file or environment
/// variables, into a structured format for easy access within the program.
///
/// # Fields
///
///`database` - Configuration details for the database connection.
///   This is represented using the `ConnectionConfig` struct.
///`server` - Configuration details for the server, such as
///   application-specific settings. This is represented using
///   the `AppConfig` struct.
///
/// # Derives
///
///`Debug` - Enables formatting of the struct using the `{:?}` syntax for debugging purposes.
///`Clone` - Allows creating duplicates of the struct.
///`Deserialize` - Allows the struct to be deserialized from formats
///   like JSON, TOML, or YAML, provided the deserialization framework supports it.
///
/// # Visibility
///
/// The struct is marked as `pub(crate)`, which means it is public to
/// the current crate but not accessible outside

#[derive(Debug, Clone, Deserialize, Serialize, Default)]
pub struct Config {
    #[serde(default)]
    database: Arc<DatabaseConfig>,
    #[serde(default)]
    app: Arc<AppConfig>,
    #[serde(default)]
    redis: Arc<RedisConfig>,
    #[serde(default)]
    licenses: Arc<Licenses>,
    #[serde(flatten)]
    ___flatten: Arc<BTreeMap<String, serde_yaml::Value>>,
    // SKIP LIBS
    #[serde(skip)]
    file: PathBuf,
    #[serde(skip)]
    session_tokenizer: OnceLock<Arc<SessionTokenizer>>,
    #[serde(skip)]
    session_manager: OnceLock<Arc<SessionManager>>,
    #[serde(skip)]
    csrf_duration: OnceLock<Arc<CsrfDuration>>,
    #[serde(skip)]
    connection: OnceLock<Arc<Connection>>,
    #[serde(skip)]
    ssl_storage: OnceLock<Arc<SSLStorage>>,
}

/// Application configuration struct.
impl AppConfig {
    pub fn socket(&self) -> &str {
        &self.socket
    }
    pub fn csrf_durations(&self) -> usize {
        self.csrf_durations
    }
    pub fn csrf_duration(&self) -> Duration {
        Duration::from_mins(self.csrf_durations as u64)
    }
    pub fn session_name(&self) -> &str {
        &self.session_name
    }
    pub fn salt_session_name(&self) -> String {
        format!("{}_salt", self.session_name())
    }
    pub fn datagram(&self) -> &str {
        &self.datagram
    }
    pub fn worker(&self) -> usize {
        self.workers
    }
    pub fn tcp(&self) -> &Vec<String> {
        &self.tcp
    }
    pub fn mode(&self) -> &str {
        &self.mode
    }
    pub fn operation_timeout(&self) -> usize {
        self.operation_timeout
    }
    pub fn operation_timeout_duration(&self) -> Duration {
        Duration::from_secs(self.operation_timeout as u64)
    }
    pub fn request_timeout(&self) -> usize {
        self.request_timeout
    }
    pub fn request_timeout_duration(&self) -> Duration {
        Duration::from_secs(self.request_timeout as u64)
    }
    pub fn disconnect_timeout(&self) -> usize {
        self.disconnect_timeout
    }
    pub fn disconnect_timeout_duration(&self) -> Duration {
        Duration::from_secs(self.disconnect_timeout as u64)
    }
    pub fn secret_key(&self) -> &str {
        &self.secret_key
    }
    pub fn salt_key(&self) -> &str {
        &self.salt_key
    }
    pub fn max_connections(&self) -> usize {
        self.max_connections
    }
    pub fn dashboard_path(&self) -> &str {
        &self.dashboard_path
    }
    pub fn max_connections_rate(&self) -> usize {
        self.max_connections_rate
    }
    pub fn backlog(&self) -> u32 {
        self.backlog
    }
    pub fn rlimit_nofile(&self) -> usize {
        self.rlimit_nofile
    }
    pub fn ssl(&self) -> &SSLConfig {
        &self.ssl
    }
    pub fn keep_alive(&self) -> usize {
        self.keep_alive
    }
    pub fn keep_alive_duration(&self) -> KeepAlive {
        let ka = self.keep_alive();
        if ka == 0 {
            return KeepAlive::Disabled;
        }
        KeepAlive::Timeout(Duration::from_secs(ka as u64))
    }
    pub fn user(&self) -> &str {
        &self.user
    }
    pub fn flatten(&self) -> Arc<BTreeMap<String, serde_yaml::Value>> {
        self.___flatten.clone()
    }
    pub fn is_production(&self) -> bool {
        let mode = self.mode().to_lowercase();
        mode.starts_with("prod")
    }
    pub fn auto_clean_memory(&self) -> bool {
        self.auto_clean_memory
    }

    pub fn auto_clean_memory_size(&self) -> &str {
        &self.auto_clean_memory_size
    }
    pub fn auto_clean_memory_size_bytes(&self) -> usize {
        Hack::size_to_bytes_zero(self.auto_clean_memory_size())
    }
    pub fn auto_clean_memory_interval(&self) -> &str {
        &self.auto_clean_memory_interval
    }
    pub fn auto_clean_memory_interval_duration(&self) -> Duration {
        Hack::string_to_duration_compat(self.auto_clean_memory_interval())
    }
}

/// Configuration loading utility function.
impl Config {
    /// Returns a flattened representation of the data as a `BTreeMap`.
    ///
    /// This method provides access to a precomputed flattened version of the data
    /// stored within the structure. The output is a `BTreeMap` where the keys are
    /// `String's representing the paths to the nested values in the original data,
    /// and the values are `serde_yaml::Value` representing those data points.
    ///
    /// # Returns
    ///
    /// A clone of the underlying `BTreeMap<String, serde_yaml::Value>` containing
    /// the flattened data structure.
    ///
    /// # Example
    ///
    /// ```rust
    /// let flattened_map = your_instance.flatten();
    /// for (key, value) in &flattened_map {
    ///     println!("{}: {:?}", key, value);
    /// }
    /// ```
    pub fn flatten(&self) -> &BTreeMap<String, serde_yaml::Value> {
        &self.___flatten
    }

    /// Returns the default configuration as a static string slice.
    ///
    /// # Description
    /// This function provides access to a globally defined default configuration
    /// stored in the constant `DEFAULT_CONFIG`. It is typically used to get
    /// the default settings for an application or library.
    ///
    /// # Returns
    /// A static string slice (`&'static str`) containing the default configuration.
    ///
    /// # Example
    /// ```
    /// let config = default_config();
    /// println!("Default Config: {}", config);
    /// ```
    ///
    /// # Notes
    /// - The `DEFAULT_CONFIG` constant must be defined elsewhere in the scope for
    ///   this function to work correctly.
    ///
    pub fn default_config() -> &'static str {
        DEFAULT_CONFIG
    }

    /// Loads the runtime configuration based on the provided console arguments.
    ///
    /// This function attempts to load the configuration file specified in the
    /// `ConsoleArguments`. If no configuration file is explicitly provided, the
    /// default configuration file determined by `Runtime::config_file()` will be used.
    /// The function handles both relative and absolute file paths. If the file
    /// does not exist or cannot be loaded, an error is returned.
    ///
    /// # Arguments
    ///c
    /// * `arg` - An optional reference to `ConsoleArguments`. If provided, it may
    ///           specify the path to the configuration file. If absent, the default
    ///           configuration file is used.
    ///
    /// # Returns
    ///
    /// * `Result<Self, Error>` - Returns an instance of `Self` with the loaded
    ///   configuration on success, or an `Error` if the configuration file cannot
    ///   be found or loaded.
    ///
    /// # Errors
    ///
    /// * Returns `Error::file_not_found` if the configuration file does not exist.
    /// * Returns any error encountered during the process of loading the configuration file.
    ///
    /// # Example
    ///
    /// ```rust
    /// let args = ConsoleArguments::new(Some("path/to/config"));
    /// match Runtime::load(Some(&args)) {
    ///     Ok(runtime) => println!("Configuration loaded successfully!"),
    ///     Err(e) => eprintln!("Failed to load configuration: {}", e),
    /// }
    /// ```
    ///
    /// # Notes
    ///
    /// This function assumes that the provided configuration path, if any, is valid.
    /// Relative paths are resolved relative to the current directory as determined by
    /// `Runtime::current_dir()`.
    pub fn load(arg: Option<&ConsoleArguments>) -> ResultError<Self> {
        let arg_c = arg.unwrap().config.clone();
        let config_file = match &arg_c {
            None => Runtime::config_file(),
            Some(c) => {
                if !c.is_absolute() {
                    &Runtime::current_dir().join(c)
                } else {
                    c.as_path()
                }
            }
        };
        if !config_file.exists() {
            return Err(Error::file_not_found(format!(
                "Config file not found: {}",
                config_file.display()
            )));
        }
        let config = Self::load_from_file(config_file)?;
        Ok(config)
    }

    /// Loads and parses a configuration file from the specified path.
    ///
    /// # Arguments
    ///`path` - A `PathBuf` representing the path to the configuration file.
    ///
    /// # Returns
    ///`Ok(Self)` - The successfully loaded and parsed `Config` object.
    ///`Err(Error)` - An error encountered during the loading or parsing of the configuration file.
    ///
    /// # Behavior
    /// This function performs the following tasks:
    /// 1. Validates if the file exists at the specified path. If not, returns an error
    ///    with `ErrorKind::NotFound`.
    /// 2. Checks if the file size exceeds 1 MB. If it does,it returns an error with
    ///    `ErrorKind::InvalidData`.
    /// 3. Reads the file contents into a string and parses it as a YAML configuration
    ///    file using `serde_yaml`.
    /// 4. Sets the `file_path` field of the resultant `Config` object to the provided `path`.
    /// 5. Verifies and adjusts the `storage_path` in the configuration:
    ///    - If the `storage_path` is relative, converts it to an absolute path
    ///      relative to the directory of the configuration file.
    /// 6. Ensures the `tcp` field in the configuration has valid TCP listeners:
    ///    - If the `tcp` listeners list is empty, initializes it with a default
    ///      listener value.
    ///    - Validates each TCP listener:
    ///      - Ensures the port is within the valid range (1024-65535).
    ///      - Normalizes `localhost` to `127.0.0.1`.
    ///      - Validates the IP address using an external IP version checker.
    ///      - Ensures no duplicate TCP listeners are configured.
    /// 7. Updates the `storage_path` to ensure it is cleaned and in absolute format.
    ///
    /// # Errors
    /// This function can encounter and return the following errors:
    /// - `ErrorKind::NotFound`: If the configuration file does not exist.
    /// - `ErrorKind::InvalidData`: If the file exceeds the maximum allowed size, YAML parsing fails,
    ///   an invalid TCP port is detected, or an invalid TCP listener is specified.
    ///
    /// # Example
    /// ```rust
    /// use std::path::PathBuf;
    ///
    /// let config_path = PathBuf::from("config.yaml");
    /// let config = load_config(&config_path);
    /// match config {
    ///     Ok(cfg) => println!("Configuration loaded successfully: {:?}", cfg),
    ///     Err(e) => eprintln!("Failed to load configuration: {}", e),
    /// }
    /// ```
    pub fn load_from_file(path: &Path) -> ResultError<Self> {
        if !path.exists() {
            return Err(Error::file_not_found(format!(
                "Config file not found: {}",
                path.display()
            )));
        }
        let max_size = 10241024; // 1 MB
        if path.metadata()?.len() > max_size {
            return Err(Error::invalid_data(format!(
                "Config file too large: {} bytes",
                path.metadata()?.len()
            )));
        }

        let content = std::fs::read_to_string(path)?;
        Ok(Self::load_from_content(&content, path)?)
    }

    pub fn is_production(&self) -> bool {
        let mode = self.app.mode().to_lowercase();
        mode.starts_with("prod")
    }

    /// Loads a configuration object from a YAML string and performs validation and normalization.
    ///
    /// # Parameters
    /// - `content`: A reference to a YAML string slice containing the configuration data.
    /// - `path`: A reference to a `PathBuf` that specifies the file path where the configuration
    ///   content originates.
    ///
    /// # Returns
    /// - A `ResultError<Self>`:
    ///   - `Ok(Self)`: If the configuration is successfully parsed, normalized, and validated.
    ///   - `Err(Error)`: If an error occurs during parsing or validation.
    ///
    /// # Behavior
    /// - Parses the input `content` as a YAML string to construct a `Config` object.
    /// - Stores the `path` to the `file_path` property of the configuration.
    /// - Resolves the `storage_path` property:
    ///   - If the path is relative, it appends it to the parent directory of `path`.
    /// - Logs information if the `storage_path` is modified to an absolute path.
    /// - Validates and resolves the `tcp` property:
    ///   - Assigns default TCP listeners (`default_server_tcp_listener()`) if the `tcp` list is empty.
    ///   - Ensures that each TCP entry has a valid port in the range [1024-65535].
    ///   - Normalizes the host for `localhost` as `127.0.0.1` and validates its IP version.
    ///   - Eliminates duplicate TCP listeners, logging occurrences.
    /// - Modifies the `storage_path` property to a cleaned-up path relative to its parent directory.
    ///
    /// # Error Conditions
    /// - Fails with `ErrorKind::InvalidData` in the following cases:
    ///   - The YAML input is malformed or cannot be deserialized into the `Config` structure.
    ///   - A TCP listener has an invalid port number or IP address format.
    ///
    /// # Logging
    /// - Logs the resolved absolute storage path.
    /// - Logs duplicate TCP listener entries found during validation.
    ///
    /// # Example Usage
    /// ```
    /// let content = r#"
    /// app:
    ///   tcp:
    ///     - "127.0.0.1:8080"
    ///   storage_path: "data/storage"
    /// "#;
    /// let path = PathBuf::from("/config/config.yaml");
    /// let config = Config::load_from_content(content, &path);
    /// match config {
    ///     Ok(c) => println!("Configuration loaded successfully: {:?}", c),
    ///     Err(e) => eprintln!("Failed to load configuration: {}", e),
    /// }
    /// ```
    ///
    /// # Remarks
    /// - This function assumes that the caller provides reliable inputs for `content` and `path`.
    /// - Ensures configuration properties comply with the required structure and values before returning.
    pub fn load_from_content(content: &str, path: &Path) -> ResultError<Self> {
        let mut parsed: Config =
            serde_yaml::from_str(&content).map_err(|e| Error::invalid_data(e))?;
        parsed.file = (*path).to_owned().clean();
        let app = Arc::make_mut(&mut parsed.app);
        Arc::make_mut(&mut parsed.database).set_event_manager(Some(Factory::pick_unsafe::<EventManager>()));
        // check tcp
        if app.tcp.is_empty() {
            app.tcp = default_server_tcp_listener();
        }
        let mut unique_tcp = HashSet::new();
        let mut unique_ssl = HashSet::new();
        // check if tcp port is valid
        for tcp in &app.tcp {
            let port: u64 = tcp
                .split(':')
                .last()
                .unwrap()
                .parse()
                .map_err(Error::parse_error)?;
            let host: &str = &tcp[..tcp.rfind(':').unwrap()];
            if port < 1 || port > 65535 {
                return Err(Error::invalid_data(format!("Invalid TCP port: {}", port)));
            }
            let normalized_host = if host.to_lowercase() == "localhost" {
                "127.0.0.1"
            } else {
                host
            };
            Ip::version(normalized_host).map_err(|_| {
                Error::invalid_data(format!("Invalid TCP Listener for {}", tcp.bold().yellow()))
            })?;
            let n_tcp = format!("{}:{}", normalized_host, port);
            if port == 443 {
                debug!(target: "factory", "Port 443 detected! moving to ssl");
                unique_ssl.insert(n_tcp);
                continue;
            }
            if !unique_tcp.insert(n_tcp.clone()) {
                trace!(target: "factory", "{} is duplicate, skipping", n_tcp.yellow());
                continue;
            }
        }
        let ssl = app.clone().ssl;
        for n in &ssl.listen {
            let port: u64 = n
                .split(':')
                .last()
                .unwrap()
                .parse()
                .map_err(Error::parse_error)?;
            let mut host: &str = &n[..n.rfind(':').unwrap()];
            if port < 1 || port > 65535 {
                return Err(Error::invalid_data(format!("Invalid TCP port: {}", port)));
            }
            if host.is_empty() {
                host = "0.0.0.0";
            }
            let normalized_host = if host.to_lowercase() == "localhost" {
                "127.0.0.1"
            } else {
                host
            };
            Ip::version(normalized_host).map_err(|_| {
                Error::invalid_data(format!("Invalid TCP Listener for {}", n.bold().yellow()))
            })?;
            let n_tcp = format!("{}:{}", normalized_host, port);
            if port == 80 {
                debug!(target: "factory", "Port 80 detected! moving to ssl");
                unique_tcp.insert(n_tcp);
                continue;
            }
            if !unique_ssl.insert(n_tcp.clone()) {
                trace!(target: "factory", "{} is duplicate, skipping", n_tcp.yellow());
                continue;
            }
        }
        app.ssl.listen = unique_ssl.into_iter().collect();
        if ssl.key.trim().is_empty() {
            app.ssl.key = "".to_string();
        }
        if ssl.key.trim().is_empty() {
            app.ssl.key = "".to_string();
        }
        if ssl.cert.trim().is_empty() {
            app.ssl.cert = "".to_string();
        }
        let session_cache = ssl.session_cache();
        if session_cache == 0 {
            app.ssl.session_cache = default_ssl_session_cache();
        } else {
            app.ssl.session_cache = app
                .ssl
                .session_cache
                .clamp(MIN_SSL_SESSION_CACHE, MAX_SSL_SESSION_CACHE);
        }
        let socket = app.socket();
        if !socket.trim().is_empty() {
            let info = FileInfo::new(socket);
            if info.is_relative() {
                app.socket = Runtime::exe_file()
                    .join(socket)
                    .to_string_lossy()
                    .to_string();
            }
        } else {
            app.socket = Runtime::socket_file().to_string_lossy().to_string();
        }
        let max_connections = app.max_connections();
        let has_limit_less = Runtime::has_limit_less();
        let u16_limitless = Runtime::u16_limitless();
        let max_connections_limit = if !has_limit_less {
            MAX_CONNECTIONS
        } else {
            u16_limitless
        };
        let max_rate_limit = if !has_limit_less {
            MAX_CONNECTION_RATE
        } else {
            u16_limitless
        };
        app.max_connections = max_connections.clamp(MIN_CONNECTIONS, max_connections_limit);
        let mut max_connections_rate = app.max_connections_rate();
        if max_connections_rate == 0 {
            max_connections_rate = default_max_connections_rate();
            app.max_connections_rate = max_connections_rate;
        } else {
            app.max_connections_rate =
                max_connections_rate.clamp(MIN_CONNECTION_RATE, max_rate_limit);
        }
        app.backlog = app.backlog().clamp(MIN_BACKLOG, MAX_BACKLOG);
        app.rlimit_nofile = app
            .rlimit_nofile()
            .clamp(MIN_RLIMIT_NOFILE as usize, MAX_RLIMIT_NOFILE as usize);
        app.operation_timeout = app
            .operation_timeout()
            .clamp(MIN_OPERATION_TIMEOUT, MAX_OPERATION_TIMEOUT);
        app.request_timeout = app
            .request_timeout()
            .clamp(MIN_REQUEST_TIMEOUT, MAX_REQUEST_TIMEOUT);
        app.disconnect_timeout = app
            .disconnect_timeout()
            .clamp(MIN_DISCONNECT_TIMEOUT, MAX_DISCONNECT_TIMEOUT);
        app.csrf_durations = app
            .csrf_durations()
            .clamp(MIN_CSRF_DURATION_MINUTES, MAX_CSRF_DURATION_MINUTES);
        app.keep_alive = app.keep_alive().clamp(MIN_KEEP_ALIVE, MAX_KEEP_ALIVE);
        let workers = app.workers;
        if workers == 0 {
            app.workers = default_workers();
        }
        let auto_clean_mb = app.auto_clean_memory_size_bytes();
        if auto_clean_mb != 0 && auto_clean_mb < MIN_AUTO_CLEAN_MEMORY_SIZE_BYTES {
            app.auto_clean_memory_size = MIN_AUTO_CLEAN_MEMORY_SIZE_BYTES.to_string();
        }
        let auto_clean_interval = app.auto_clean_memory_interval_duration().as_secs() as usize;
        if auto_clean_interval > MAX_AUTO_CLEAN_MEMORY_INTERVAL {
            app.auto_clean_memory_interval = MAX_AUTO_CLEAN_MEMORY_INTERVAL.to_string();
        }
        if auto_clean_interval < MIN_AUTO_CLEAN_MEMORY_INTERVAL {
            app.auto_clean_memory_interval = MIN_AUTO_CLEAN_MEMORY_INTERVAL.to_string();
        }
        let max_worker = std::thread::available_parallelism()
            .map(|n| n.get())
            .unwrap_or(1)
            * 5; // max worker is 5 x of cpu (RUST IS LIGHTWEIGHT!)
        if app.workers > max_worker {
            warn!(
                "Worker limit overhead {}, allowed worker maximum is: {}",
                app.workers, max_worker
            );
            app.workers = max_worker;
        }

        let dashboard_path = app.dashboard_path().trim_start_matches('/').trim_end_matches('/');
        if dashboard_path.is_empty() {
            warn!(target: "factory:config", "Dashboard path is empty fallback default: {}", DEFAULT_DASHBOARD_PATH);
            app.dashboard_path = DEFAULT_DASHBOARD_PATH.to_string();
        } else {
            let mut new_dashboard_path = dashboard_path.to_lowercase();
            new_dashboard_path = RE_DASHBOARD_PATH_UNWANTED_CHARS.replace_all(&new_dashboard_path, "").to_string();
            if new_dashboard_path.is_empty() {
                warn!(target: "factory:config", "Dashboard path is empty fallback default: {}", DEFAULT_DASHBOARD_PATH);
                new_dashboard_path = DEFAULT_DASHBOARD_PATH.to_string();
            } else if !RE_DASHBOARD_PATH.is_match(&new_dashboard_path) {
                warn!(target: "factory:config", "Dashboard path [{}] is invalid fallback default: {}", new_dashboard_path, DEFAULT_DASHBOARD_PATH);
                new_dashboard_path = DEFAULT_DASHBOARD_PATH.to_string();
            }
            app.dashboard_path = new_dashboard_path.to_string();
        }
        app.socket = socket_file(); // static
        app.datagram = datagram_identity(); // static
        app.tcp = unique_tcp.into_iter().collect();
        let mut session_name = app.session_name().trim().to_string();
        if session_name.is_empty() {
            session_name = default_session_name();
        }
        app.session_name = session_name;
        if app.user.is_empty() // check if empty
            || app.user.trim().is_empty() // check if empty
            || User::from_name(&app.user.trim()).is_root()
        // check if root
        {
            app.user = default_user();
        }
        // reconfigure
        Ok(parsed)
    }

    pub fn file(&self) -> &Path {
        &self.file
    }

    /// Returns a reference to the server's configuration.
    ///
    /// This method provides access to the `AppConfig` instance associated with the server,
    /// allowing retrieval of configuration details.
    ///
    /// # Returns
    ///
    /// A reference to the `AppConfig` instance.
    ///
    /// # Examples
    ///
    /// ```
    /// let config = server.server();
    /// ```
    pub fn app(&self) -> Arc<AppConfig> {
        Arc::clone(&self.app)
    }

    /// Returns a reference to the database connection configuration.
    ///
    /// # Returns
    /// A reference to the `ConnectionConfig` struct, which contains the
    /// configuration details for the database connection.
    ///
    /// # Example
    /// ```rust
    /// let config = my_object.database();
    /// // Use `config` to access database connection configuration details.
    /// ```
    ///
    /// This method provides read-only access to the database connection configuration
    /// stored within the current instance of the struct.
    pub fn database(&self) -> Arc<DatabaseConfig> {
        Arc::clone(&self.database)
    }

    pub fn redis(&self) -> Arc<RedisConfig> {
        Arc::clone(&self.redis)
    }

    pub fn get_socket(&self) -> ResultError<String> {
        let app = self.app();
        let socket = app.socket();
        if socket.trim().is_empty() {
            return Err(Error::address_not_available(
                "Socket configuration is empty",
            ));
        }
        Ok(socket.to_string())
    }

    /// Retrieves the identity of the datagram as a string.
    ///
    /// # Returns
    ///
    /// * `Ok(String)` - A string representation of the datagram identity.
    /// * `Err(Error)` - If an error occurs during the retrieval process.
    ///
    /// This method accesses the `app` instance, retrieves the `datagram`
    /// object, and converts it to a `String`.
    ///
    /// # Example
    /// ```rust
    /// let identity = instance.get_datagram_identity();
    /// match identity {
    ///     Ok(id) => println!("Datagram identity: {}", id),
    ///     Err(e) => eprintln!("Failed to get datagram identity: {:?}", e),
    /// }
    /// ```
    ///
    /// # Errors
    /// This method returns an error if there is an issue accessing the
    /// underlying `app` or `datagram` functionality.
    pub fn get_datagram_identity(&self) -> String {
        self.app().datagram().to_string()
    }

    /// Retrieves the identity of the datagram server.
    ///
    /// This method combines the datagram identity of the server with the current process ID to generate
    /// a unique identifier for the server instance. The identity is fetched using the `get_datagram_identity`
    /// method, which is expected to be implemented in the same structure.
    ///
    /// # Returns
    ///
    /// A `String` representing the datagram server's identity. The format of the returned string is:
    /// `<datagram_identity>/server`, where:
    /// - `<datagram_identity>` is obtained from `self.get_datagram_identity()`.
    /// - `/server` is appended to the identity string.
    ///
    /// # Example
    ///
    /// ```
    /// let server_identity = server.get_datagram_server_identity();
    /// println!("Datagram Server Identity: {}", server_identity);
    /// ```
    ///
    /// # Notes
    ///
    /// - This method assumes the existence of a `get_datagram_identity` method within the same structure.
    /// - The returned identity is unique to the combination of the server and the running process instance.
    pub fn get_datagram_server_identity(&self) -> String {
        let identity = self.get_datagram_identity();
        format!("{}/server", identity)
    }

    /// Generates a unique client identity string for the datagram client.
    ///
    /// This function creates a unique identifier for the datagram client by
    /// combining the client's identity obtained with `get_datagram_identity`
    /// and the process ID of the current running process. The format of the
    /// returned string is:
    ///
    /// `<identity>/client/<pid>`
    ///
    /// # Returns
    ///
    /// A `String` representing the unique client identity.
    ///
    /// # Example
    ///
    /// ```
    /// let client_identity = instance.get_datagram_client_identity();
    /// println!("{}", client_identity); // Output might look like "some_identity/client/12345"
    /// ```
    ///
    /// # Notes
    ///
    /// - The process ID is fetched using `std::process::id()`.
    /// - The identity provided by `get_datagram_identity` must be implemented
    ///   and available in the context of this function.
    pub fn get_datagram_client_identity(&self) -> String {
        let identity = self.get_datagram_identity();
        let pid = std::process::id();
        format!("{}/client/{}", identity, pid)
    }

    /// Creates a `SocketAddr` for a datagram connection based on the instance's identity.
    ///
    /// This function retrieves the datagram's identifying information and uses it to
    /// construct a `SocketAddr` object. It leverages the `from_abstract_name` method to
    /// convert the identity string into a socket address. If the identity is invalid or
    /// the conversion fails, an error is returned with a descriptive message.
    ///
    /// # Returns
    ///
    /// - `Ok(SocketAddr)` if the datagram identity is successfully converted into a `SocketAddr`.
    /// - `Err(Error)` if the conversion fails due to invalid input or another error.
    ///
    /// # Errors
    ///
    /// This function returns an `Error` in the following situations:
    /// - The datagram identity string is invalid.
    /// - The socket address conversion (`from_abstract_name`) fails.
    ///
    /// # Examples
    ///
    /// ```rust
    /// let instance = MyDatagramInstance::new();
    /// match instance.create_datagram_address() {
    ///     Ok(addr) => println!("Datagram address created: {}", addr),
    ///     Err(e) => eprintln!("Failed to create datagram address: {}", e),
    /// }
    /// ```
    ///
    /// # Notes
    ///
    /// - The `get_datagram_identity` method is assumed to retrieve a string representation
    ///   of the instance's unique identity used for creating the `SocketAddr`.
    /// - Ensure that the `socket_control` string is valid and properly formatted for use
    ///   with `SocketAddr::from_abstract_name`.
    pub fn create_datagram_server_address(&self) -> ResultError<SocketAddr> {
        let socket_control = self.get_datagram_server_identity();
        SocketAddr::from_abstract_name(socket_control.as_bytes()).map_err(|e| {
            Error::other(format!(
                "Invalid socket server control @{}: {}",
                socket_control, e
            ))
        })
    }

    /// Creates and returns a `SocketAddr` for the datagram client, uniquely
    /// identified by the current process ID (PID).
    ///
    /// # Steps
    /// - Retrieves the current process ID using `std::process::id`.
    /// - Constructs an abstract name for the socket control path using the
    ///   datagram identity and the PID.
    /// - Attempts to create a `SocketAddr` from the constructed abstract name.
    ///
    /// # Returns
    /// - `Ok(SocketAddr)` if the address creation succeeds.
    /// - `Err(Error)` if the address creation fails, capturing details of the
    ///   invalid socket client control and the underlying error.
    ///
    /// # Errors
    /// Returns an `Error` with:
    /// - The kind set to `ErrorKind::Other`.
    /// - A descriptive log of the invalid socket control and the cause of the
    ///   failure.
    ///
    /// # Examples
    /// ```
    /// let address = instance.create_datagram_client_address();
    /// match address {
    ///     Ok(socket_addr) => println!("Successfully created client address: {:?}", socket_addr),
    ///     Err(e) => eprintln!("Failed to create client address: {}", e),
    /// }
    /// ```
    pub fn create_datagram_client_address(&self) -> ResultError<SocketAddr> {
        let socket_control = self.get_datagram_client_identity();
        SocketAddr::from_abstract_name(&socket_control.as_bytes()).map_err(|e| {
            Error::other(format!(
                "Invalid socket client control @{}: {}",
                socket_control, e
            ))
        })
    }

    /// Creates a unique address for a datagram client by generating a UUIDv7
    /// and combining it with the client's identity.
    ///
    /// # Description
    /// This function generates a unique identifier by combining the client's
    /// identity (retrieved using `get_datagram_client_identity()`) with a
    /// UUIDv7. The resulting string is used to create an abstract socket
    /// address, ensuring uniqueness for the datagram client.
    ///
    /// # Returns
    /// - `Ok(SocketAddr)` containing the newly created abstract socket address
    ///   derived from the client's identity and the generated UUIDv7.
    /// - `Err(Error)` if the string format of the abstract name is invalid or
    ///   conversion to a socket address fails.
    ///
    /// # Errors
    /// Returns an error of type `ErrorKind::Other` if:
    /// - The combined string for the abstract name is invalid.
    /// - The conversion to a `SocketAddr` fails.
    ///
    /// The error will include a descriptive message about the invalid input
    /// and the underlying cause.
    ///
    /// # Examples
    /// ```
    /// let address = client.create_datagram_client_unique_address();
    /// match address {
    ///     Ok(addr) => println!("Unique address created: {}", addr),
    ///     Err(e) => eprintln!("Failed to create unique address: {}", e),
    /// }
    /// ```
    pub fn create_datagram_client_unique_address(&self) -> ResultError<SocketAddr> {
        let uuid_v7 = Uuid::v7();
        let socket_control = format!("{}/{}", self.get_datagram_client_identity(), uuid_v7);
        SocketAddr::from_abstract_name(&socket_control.as_bytes()).map_err(|e| {
            Error::other(format!(
                "Invalid socket client control @{}: {}",
                socket_control, e
            ))
        })
    }

    /// Creates a `SocketAddr` for the server socket.
    ///
    /// This function retrieves the socket name from `self` using the `get_socket` method
    /// and attempts to create a valid `SocketAddr` using `SocketAddr::from_abstract_name`.
    /// If the operation fails, it returns an error with details about the invalid socket.
    /// This is suitable for bulletproof multi-process server control communication.
    /// # Returns
    /// * `Ok(SocketAddr)` - If the socket address is successfully created.
    /// * `Err(Error)` - If there is an error retrieving the socket or constructing
    ///   the socket address, an `Error` is returned with additional context.
    ///
    /// # Errors
    /// Returns an error in the following cases:
    /// * When `get_socket()` fails to retrieve a valid socket.
    /// * When `SocketAddr::from_abstract_name` fails to generate a valid `SocketAddr`.
    ///
    /// The error message includes the offending socket name and the encountered problem.
    ///
    /// # Example
    /// ```rust
    /// let socket_addr = my_instance.create_server_socket_address();
    /// match socket_addr {
    ///     Ok(addr) => println!("Server socket address created: {}", addr),
    ///     Err(e) => eprintln!("Failed to create server socket address: {}", e),
    /// }
    /// ```
    pub fn create_server_socket_address(&self) -> ResultError<SocketAddr> {
        let socket = self.get_socket()?;
        SocketAddr::from_abstract_name(&socket)
            .map_err(|e| Error::other(format!("Invalid socket client control @{}: {}", socket, e)))
    }

    pub fn get_session_tokenizer(&self) -> Arc<SessionTokenizer> {
        self.session_tokenizer
            .get_or_init(|| {
                let app = self.app();
                Arc::new(SessionTokenizer::new(app.secret_key(), app.salt_key()))
            })
            .clone()
    }

    pub fn get_csrf_duration(&self) -> Arc<CsrfDuration> {
        self.csrf_duration
            .get_or_init(|| {
                let app = self.app();
                Arc::new(CsrfDuration::new(
                    app.secret_key(),
                    app.salt_key(),
                    Some(app.csrf_duration()),
                ))
            })
            .clone()
    }

    pub fn get_session_manager(&self) -> Arc<SessionManager> {
        self.session_manager
            .get_or_init(|| {
                let app = self.app();
                Arc::new(SessionManager::new(
                    app.session_name(),
                    self.get_session_tokenizer()
                ))
            })
            .clone()
    }

    pub fn get_connection(&self) -> Arc<Connection> {
        self.connection
            .get_or_init(|| {
                Arc::new(self.database().to_connection())
            })
            .clone()
    }

    pub fn get_ssl_storage(&self) -> Arc<SSLStorage> {
        self.ssl_storage.get_or_init(||{
            Arc::new(SSLStorage::new(self.app().ssl.clone()))
        }).clone()
    }
}
