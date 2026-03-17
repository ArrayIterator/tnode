use crate::cores::database::adapter::{Adapter, Driver};
use crate::cores::database::connection::Connection;
use crate::cores::helper::log_level::LogLevel;
use crate::cores::system::event_manager::EventManager;
use log::LevelFilter;
use serde::{Deserialize, Serialize};
use sqlx::any::{AnyConnectOptions, AnyPoolOptions};
use sqlx::{AnyPool, ConnectOptions, Database, MySql, Postgres, Sqlite};
use std::any::TypeId;
use std::collections::BTreeMap;
use std::sync::Arc;
use std::time::Duration;
use url::Url;

pub const MAX_CONNECTIONS: u32 = 500; // max connections
pub const MIN_CONNECTIONS: u32 = 5; // min connections
pub const MAX_LIFETIME: u64 = 7200; // max lifetime - 2 hours is a very long time
pub const MIN_LIFETIME: u64 = 30;
pub const MIN_IDLE_TIMEOUT: u64 = 10; // min idle timeout - 10 seconds is a very short time
pub const MAX_IDLE_TIMEOUT: u64 = 3600; // max idle timeout - 1 hour is a very long time
pub const MIN_ACQUIRE_TIMEOUT: u64 = 1; // min acquire timeout - 1 second is a very short time
pub const MAX_ACQUIRE_TIMEOUT: u64 = 60; // max acquire timeout - 1 minute is a very long time
pub const MIN_LOG_THRESHOLD: u64 = 1;
pub const MAX_LOG_THRESHOLD: u64 = 1000000;
pub const DEFAULT_LOG_LEVEL: &str = "info";
pub const DEFAULT_LOG_SLOW_LEVEL: &str = "warn";
pub const MAX_STATEMENT_TIMEOUT: u64 = 3600;
pub const MIN_STATEMENT_TIMEOUT: u64 = 1;
pub const DEFAULT_HOST: &str = "localhost";
pub const DEFAULT_MAX_CONNECTIONS: u32 = 50;
pub const DEFAULT_MIN_CONNECTIONS: u32 = 5;
pub const DEFAULT_MAX_LIFETIME: u64 = 1800;
pub const DEFAULT_MIN_LIFETIME: u64 = 30;
pub const DEFAULT_IDLE_TIMEOUT: u64 = 600;
pub const DEFAULT_ACQUIRE_TIMEOUT: u64 = 10;
pub const DEFAULT_STATEMENT_TIMEOUT: u64 = 5;
pub const DEFAULT_LOG_THRESHOLD: u64 = 1;
pub const DEFAULT_DRIVER: &str = Postgres::NAME;
pub const DEFAULT_COLLATION: &str = "utf8mb4_general_ci";

#[derive(Debug, Clone, Serialize)]
pub struct Configuration {
    #[serde(default)]
    pub socket: Option<String>,
    #[serde(default)]
    pub driver: String,
    #[serde(default)]
    pub host: String,
    #[serde(default)]
    pub port: u16,
    #[serde(default)]
    pub username: String,
    #[serde(default)]
    pub password: String,
    #[serde(default)]
    pub database: String,
    #[serde(default)]
    pub max_connections: u32,
    #[serde(default)]
    pub min_connections: u32,
    #[serde(default)]
    pub max_lifetime: u64,
    #[serde(default)]
    pub statement_timeout: u64,
    #[serde(default)]
    pub idle_timeout: u64,
    #[serde(default)]
    pub acquire_timeout: u64,
    #[serde(default)]
    pub log_enable: bool,
    #[serde(default)]
    pub log_level: String,
    #[serde(default)]
    pub log_slow_level: String,
    #[serde(default)]
    pub log_threshold: u64,
    #[serde(default)]
    pub collation: String,
    #[serde(default)]
    is_default_port: bool,
    #[serde(flatten)]
    ___flatten: BTreeMap<String, serde_yaml::Value>,
    #[serde(skip)]
    event_manager: Option<Arc<EventManager>>,
}

impl Default for Configuration {
    fn default() -> Self {
        Self {
            socket: None,
            driver: DEFAULT_DRIVER.to_string(),
            host: DEFAULT_HOST.to_string(),
            port: 0,
            username: "".to_string(),
            password: "".to_string(),
            database: "".to_string(),
            max_connections: DEFAULT_MAX_CONNECTIONS,
            min_connections: DEFAULT_MIN_CONNECTIONS,
            max_lifetime: DEFAULT_MAX_LIFETIME,
            idle_timeout: DEFAULT_IDLE_TIMEOUT,
            acquire_timeout: DEFAULT_ACQUIRE_TIMEOUT,
            statement_timeout: DEFAULT_STATEMENT_TIMEOUT,
            log_enable: DEFAULT_LOG_LEVEL != "off",
            log_level: DEFAULT_LOG_LEVEL.to_string(),
            log_slow_level: DEFAULT_LOG_SLOW_LEVEL.to_string(),
            log_threshold: DEFAULT_LOG_THRESHOLD,
            collation: DEFAULT_COLLATION.to_string(),
            is_default_port: true,
            ___flatten: BTreeMap::new(),
            event_manager: None,
        }
    }
}

impl<'de> Deserialize<'de> for Configuration {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        #[derive(Deserialize)]
        struct Shadow {
            #[serde(default)]
            pub socket: Option<String>,
            #[serde(default)]
            pub driver: String,
            #[serde(default)]
            pub host: String,
            #[serde(default)]
            pub port: u16,
            #[serde(default)]
            pub username: String,
            #[serde(default)]
            pub password: String,
            #[serde(default)]
            pub database: String,
            #[serde(default)]
            pub max_connections: u32,
            #[serde(default)]
            pub min_connections: u32,
            #[serde(default)]
            pub max_lifetime: u64,
            #[serde(default)]
            pub statement_timeout: u64,
            #[serde(default)]
            pub idle_timeout: u64,
            #[serde(default)]
            pub acquire_timeout: u64,
            #[serde(default)]
            pub log_enable: bool,
            #[serde(default)]
            pub log_level: String,
            #[serde(default)]
            pub log_slow_level: String,
            #[serde(default)]
            pub log_threshold: u64,
            #[serde(default)]
            pub collation: String,
            #[serde(default)]
            is_default_port: bool,
            #[serde(flatten)]
            ___flatten: BTreeMap<String, serde_yaml::Value>,
        }
        impl Default for Shadow {
            fn default() -> Self {
                Self {
                    socket: None,
                    driver: DEFAULT_DRIVER.to_string(),
                    host: DEFAULT_HOST.to_string(),
                    port: 0,
                    username: "".to_string(),
                    password: "".to_string(),
                    database: "".to_string(),
                    max_connections: DEFAULT_MAX_CONNECTIONS,
                    min_connections: DEFAULT_MIN_CONNECTIONS,
                    max_lifetime: DEFAULT_MAX_LIFETIME,
                    idle_timeout: DEFAULT_IDLE_TIMEOUT,
                    acquire_timeout: DEFAULT_ACQUIRE_TIMEOUT,
                    statement_timeout: DEFAULT_STATEMENT_TIMEOUT,
                    log_enable: DEFAULT_LOG_LEVEL != "off",
                    log_level: DEFAULT_LOG_LEVEL.to_string(),
                    log_slow_level: DEFAULT_LOG_SLOW_LEVEL.to_string(),
                    log_threshold: DEFAULT_LOG_THRESHOLD,
                    collation: DEFAULT_COLLATION.to_string(),
                    is_default_port: true,
                    ___flatten: BTreeMap::new(),
                }
            }
        }
        let config = Shadow::deserialize(deserializer)?;
        let mut config = Configuration {
            socket: config.socket,
            driver: Configuration::sanitize_driver(&config.driver, config.port),
            host: config.host,
            port: config.port,
            username: config.username,
            password: config.password,
            database: config.database,
            max_connections: config.max_connections,
            min_connections: config.min_connections,
            max_lifetime: config.max_lifetime,
            idle_timeout: config.idle_timeout,
            statement_timeout: config.statement_timeout,
            acquire_timeout: config.acquire_timeout,
            log_enable: config.log_enable,
            log_level: config.log_level,
            log_slow_level: config.log_slow_level,
            log_threshold: config.log_threshold,
            is_default_port: config.is_default_port || config.port == 0,
            collation: config.collation,
            ___flatten: config.___flatten,
            event_manager: None,
        };
        config.__reconfigure();
        Ok(config)
    }
}

impl Configuration {
    /// Sanitizes the database driver name based on the provided driver string and port number.
    /// This function takes a driver name as input and normalizes it to a standard format based on common substrings and port numbers associated with popular database types. The sanitization process includes:
    /// - Converting the input driver string to lowercase for case-insensitive comparison.
    /// - Checking for specific substrings in the driver name to identify the database type (e.g., "pg" or "po" for Postgres, "my" or "maria" for MySQL, "lite" for SQLite).
    pub fn sanitize_driver(driver: &str, port: u16) -> String {
        let mut driver = driver.to_lowercase();
        if driver.contains("pg") || driver.contains("po") {
            driver = Postgres::NAME.to_string();
        } else if driver.contains("my") || driver.contains("maria") {
            driver = MySql::NAME.to_string();
        } else if driver.contains("lite") {
            driver = Sqlite::NAME.to_string();
        } else {
            if port != 0 {
                driver = match port {
                    5432 => Postgres::NAME.to_string(),
                    3306 => MySql::NAME.to_string(),
                    _ => driver,
                };
            } else {
                driver = DEFAULT_DRIVER.to_string();
            }
        }
        driver
    }

    /// Returns the default port number for a given database type `T`.
    /// This function checks the type of the database `T` and returns the corresponding default port number:
    /// - For `Postgres`, it returns `5432`.
    /// - For `sqlx::MySql`, it returns `3306`.
    /// - For any other database type, it returns `0`.
    /// # Type Parameters
    /// - `T`: A type that implements the `Database` trait, representing a specific database type.
    /// # Returns
    /// - `u16`: The default port number associated with the database type `T`. If the type does not match known database types, it returns `0`.
    pub fn port_default_for<T: Database>() -> u16 {
        let type_id = TypeId::of::<T>();
        if type_id == TypeId::of::<Postgres>() {
            return 5432;
        }
        if type_id == TypeId::of::<sqlx::MySql>() {
            return 3306;
        }
        0
    }

    pub fn driver(&self) -> Driver {
        match self.driver.to_lowercase().as_str() {
            Postgres::NAME => Driver::Postgresql,
            MySql::NAME => Driver::MySql,
            Sqlite::NAME => Driver::Sqlite,
            _ => Driver::Postgresql, // default to Postgres if unknown
        }
    }

    /// Returns a flattened representation of the data structure as a `BTreeMap`.
    ///
    /// # Description
    /// This function retrieves a cloned version of the `___flatten` field,
    /// which is a `BTreeMap` where the keys are `String` and the values are
    /// `serde_yaml::Value`. The `flatten` function is useful for accessing
    /// the underlying representation of the data in a format that is both
    /// ordered and easy to work with.
    ///
    /// # Returns
    /// A `BTreeMap<String, serde_yaml::Value>` containing the flattened
    /// representation of the data.
    ///
    /// # Example
    /// ```rust
    /// let flattened_map = some_instance.flatten();
    /// for (key, value) in flattened_map {
    ///     println!("{}: {:?}", key, value);
    /// }
    /// ```
    ///
    /// # Notes
    /// - The returned map is a clone of the internal `___flatten` field.
    /// - Ensure that the `___flatten` field has been properly initialized
    ///   before calling this method, as this function does not modify or validate it.
    pub fn flatten(&self) -> BTreeMap<String, serde_yaml::Value> {
        self.___flatten.clone()
    }

    /// Converts the current instance into a `Connection` object.
    ///
    /// This method consumes the current instance and initializes a new `Connection`
    /// using the instance itself. It is used to transition from the current type
    /// to a `Connection` type by calling the `Connection::new` constructor.
    ///
    /// # Returns
    ///
    /// A new `Connection` object initialized with the current instance.
    ///
    /// # Example
    /// ```
    /// let instance = MyType::new();
    /// let connection = instance.into_connection();
    /// ```
    ///
    /// # Note
    /// This method consumes the current `self`, rendering it unusable after the call.
    pub fn into_connection(self) -> Connection {
        Connection::from(self)
    }

    /// Converts the current instance of the type (wrapped in an `Arc`) into a `Connection` object.
    ///
    /// This method creates a new `Connection` by cloning the `Arc` reference to ensure shared ownership
    /// of the underlying data without transferring or invalidating the original reference.
    ///
    /// # Returns
    ///
    /// A `Connection` object that shares ownership of the underlying data encapsulated in the `Arc<Self>`.
    ///
    /// # Notes
    ///
    /// This method assumes that the `Connection` type has been implemented to support such
    /// a wrapping operation via its `new` method.
    pub fn to_connection(self: &Arc<Self>) -> Connection {
        Connection::from(self.clone())
    }

    /// Creates a new instance of the type, using the default values.
    ///
    /// This function initializes the type by delegating to the `Default` trait's
    /// implementation for the type. It's a convenience method for creating a
    /// default instance without explicitly calling `Default::default()`.
    ///
    /// # Returns
    ///
    /// A new instance of the type with its default values.
    ///
    /// # Example
    ///
    /// ```
    /// let instance = MyType::new();
    /// ```
    pub fn new() -> Self {
        let mut d = Self::default();
        d.__reconfigure();
        d
    }

    /// Reconfigures the current connection settings with validation and default values.
    ///
    /// This function ensures the integrity and correctness of configuration parameters
    /// by applying the following steps:
    ///
    /// - Removes the `socket` if it exists and the `host` field is empty.
    /// - Trims and cleans up string fields (`host`, `username`, and `database`).
    /// - Validates and clamps the `port` value to a range between `1` and `65535`.
    /// - Ensures logging levels (`log_level` and `log_slow_level`) are set to valid
    ///   defaults if they are invalid or unspecified, converting them to string representations.
    /// - Sets the `port` to a default value of `5432` if it is `0`.
    /// - Provides a default username using the `default_user()` function
    ///   if the `username` is empty after trimming.
    /// - Validates the `log_threshold` ensuring it lies within `MIN_LOG_THRESHOLD`
    ///   and `MAX_LOG_THRESHOLD`.
    /// - Clamps `max_connections`, `min_connections`, and their interdependencies
    ///   within valid ranges (`MIN_CONNECTIONS` to `MAX_CONNECTIONS`).
    /// - Validates the maximum lifetime (`max_lifetime`) ensuring it
    ///   lies between `MIN_LIFETIME` and `MAX_LIFETIME`.
    /// - Ensures the `idle_timeout` value lies within bounds (`MIN_IDLE_TIMEOUT`
    ///   to `MAX_IDLE_TIMEOUT`).
    /// - Ensures the `acquire_timeout` value is clamped between
    ///   `MIN_ACQUIRE_TIMEOUT` and `MAX_ACQUIRE_TIMEOUT`.
    ///
    /// All configuration fields are adjusted based on the provided constants for
    /// their respective ranges and defaults, ensuring a consistent state is maintained.
    ///
    /// Note:
    /// - `MIN_*` and `MAX_*` constants must be defined for each configuration field.
    /// - The `LogLevel` type is expected to provide methods for managing log level transformations.
    fn __reconfigure(&mut self) {
        if self.socket.is_some() && self.host.is_empty() {
            self.socket = None;
        }
        self.host = self.host.trim().to_string();
        self.port = self.port.clamp(1, 65535);
        self.username = self.username.trim().to_string();
        self.database = self.database.trim().to_string();
        let log_level = LogLevel::level_filter_default(self.log_level.as_str(), LevelFilter::Info);
        self.log_level = LogLevel::level_string(log_level).to_string();
        let log_slow_level =
            LogLevel::level_filter_default(self.log_slow_level.as_str(), LevelFilter::Warn);
        self.log_slow_level = LogLevel::level_string(log_slow_level).to_string();
        if self.port == 0 {
            self.port = 5432;
            self.is_default_port = true;
        }
        if self.host.trim().is_empty() {
            self.host = DEFAULT_HOST.to_string();
        }
        if self.username.trim().is_empty() {
            self.username = "".to_string();
        }
        self.statement_timeout = self
            .statement_timeout
            .clamp(MIN_STATEMENT_TIMEOUT, MAX_STATEMENT_TIMEOUT);
        self.log_threshold = self
            .log_threshold
            .clamp(MIN_LOG_THRESHOLD, MAX_LOG_THRESHOLD);
        self.max_connections = self.max_connections.clamp(MIN_CONNECTIONS, MAX_CONNECTIONS);
        self.min_connections = self.min_connections.clamp(MIN_CONNECTIONS, MAX_CONNECTIONS);
        self.max_lifetime = self.max_lifetime.clamp(MIN_LIFETIME, MAX_LIFETIME);
        self.idle_timeout = self.idle_timeout.clamp(MIN_IDLE_TIMEOUT, MAX_IDLE_TIMEOUT);
        self.acquire_timeout = self
            .acquire_timeout
            .clamp(MIN_ACQUIRE_TIMEOUT, MAX_ACQUIRE_TIMEOUT);
        self.driver = Self::sanitize_driver(&self.driver, self.port);
        let collation = self.collation.trim().to_lowercase();
        // replace space & dash with underscore & trim if contains space / underscore
        let collation = collation
            .replace(' ', "_")
            .replace('-', "_")
            .replace("__", "_")
            .trim()
            .to_string();
        self.collation = if collation.is_empty() {
            DEFAULT_COLLATION.to_string()
        } else {
            collation
        };
    }

    pub fn collation(&self) -> String {
        self.collation.clone()
    }
    pub fn socket(&self) -> Option<String> {
        self.socket.clone()
    }
    pub fn host(&self) -> String {
        self.host.clone()
    }
    pub fn port(&self) -> u16 {
        self.port
    }
    pub fn username(&self) -> String {
        self.username.clone()
    }
    pub fn password(&self) -> String {
        self.password.clone()
    }
    pub fn database(&self) -> String {
        self.database.clone()
    }
    pub fn statement_timeout(&self) -> u64 {
        self.statement_timeout
    }
    pub fn max_connections(&self) -> u32 {
        self.max_connections
    }
    pub fn min_connections(&self) -> u32 {
        self.min_connections
    }
    pub fn max_lifetime(&self) -> u64 {
        self.max_lifetime
    }
    pub fn max_lifetime_duration(&self) -> Duration {
        Duration::from_secs(self.max_lifetime)
    }
    pub fn idle_timeout(&self) -> u64 {
        self.idle_timeout
    }
    pub fn idle_timeout_duration(&self) -> Duration {
        Duration::from_secs(self.idle_timeout)
    }
    pub fn acquire_timeout(&self) -> u64 {
        self.acquire_timeout
    }
    pub fn acquire_timeout_duration(&self) -> Duration {
        Duration::from_secs(self.acquire_timeout)
    }
    pub fn log_enable(&self) -> bool {
        self.log_enable
    }
    pub fn log_level(&self) -> String {
        self.log_level.clone()
    }
    pub fn log_level_filter(&self) -> LevelFilter {
        LogLevel::level(&self.log_level)
    }
    pub fn log_slow_level(&self) -> String {
        self.log_slow_level.clone()
    }
    pub fn log_slow_level_filter(&self) -> LevelFilter {
        LogLevel::level(&self.log_slow_level)
    }
    pub fn log_threshold(&self) -> u64 {
        self.log_threshold
    }
    pub fn log_threshold_duration(&self) -> Duration {
        Duration::from_secs(self.log_threshold)
    }

    /// Converts the current object into an ` Url `.
    /// If the object is not properly configured, it creates a temporary clone of itself,
    /// reconfigures that clone, and then builds and returns the `Url` from the reconfigured instance.
    /// If the object is already configured, it directly builds and returns the `Url`.
    ///
    /// # Returns
    /// A `Url` representation of the current object's state.
    ///
    /// # Behavior
    /// - If `self.configured` is `false`, the function:
    ///   1. Clones the current object into a temporary instance.
    ///   2. Reconfigures the temporary instance by calling `reconfigure()`.
    ///   3. Builds and returns a `Url` from the reconfigured instance using `build_url()`.
    /// - If `self.configured` is `true`, the function directly calls `build_url()` on the current instance.
    ///
    /// # Examples
    /// ```rust
    /// let my_object = MyObject::new();
    /// let url = my_object.as_url();
    /// println!("Generated URL: {}", url);
    /// ```
    ///
    /// # Notes
    /// - It's assumed that the `reconfigure()` and `build_url()` methods are implemented for the struct
    ///   and modify or utilize its internal fields as necessary.
    /// - The `Url` returned must be a valid URL; ensure that the relevant configuration satisfies URL creation.
    pub fn as_url(&self) -> Url {
        self.build_url()
    }

    pub fn pool(&self) -> AnyPool {
        AnyPool::connect_lazy_with(self.connect_options())
    }

    pub fn connect_options(&self) -> AnyConnectOptions {
        let url = match self.driver() {
            Driver::Postgresql => Postgres::connect_options(self).to_url_lossy(),
            Driver::MySql => MySql::connect_options(self).to_url_lossy(),
            Driver::Sqlite => Sqlite::connect_options(self).to_url_lossy(),
        };
        let mut any = AnyConnectOptions::from_url(&url).unwrap();
        any.log_settings.slow_statements_level = self.log_slow_level_filter();
        any.log_settings.slow_statements_duration = self.log_threshold_duration();
        any.log_settings.statements_level = if self.log_enable() {
            self.log_level_filter()
        } else {
            LevelFilter::Off
        };
        any
    }

    pub fn pool_options(&self) -> AnyPoolOptions {
        AnyPoolOptions::new()
            .acquire_timeout(self.acquire_timeout_duration())
            .max_connections(self.max_connections())
            .min_connections(self.min_connections())
            .max_lifetime(self.max_lifetime_duration())
            .idle_timeout(self.idle_timeout_duration())
            .acquire_slow_level(self.log_slow_level_filter())
            .acquire_slow_threshold(self.log_threshold_duration())
    }

    pub fn event_manager(&self) -> Option<Arc<EventManager>> {
        self.event_manager.as_ref().map(|e| e.clone())
    }
    pub fn set_event_manager(&mut self, event_manager: Option<Arc<EventManager>>) {
        self.event_manager = event_manager
    }
    pub fn build_url(&self) -> Url {
        self.connect_options().to_url_lossy()
    }
}
