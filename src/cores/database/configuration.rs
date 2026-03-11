use crate::cores::database::connection::{Connection, ConnectionPool};
use crate::cores::helper::log_level::LogLevel;
use crate::cores::system::event_manager::EventManager;
use log::LevelFilter;
use nix::unistd::{getuid, User};
use serde::{Deserialize, Serialize};
use sqlx::pool::PoolOptions;
use sqlx::postgres::{PgConnectOptions, PgPoolOptions};
use sqlx::{ConnectOptions, Postgres};
use std::collections::BTreeMap;
use std::sync::{Arc, OnceLock};
use std::time::Duration;
use url::Url;

pub fn default_host() -> String {
    "localhost".to_string()
}
pub fn default_socket() -> Option<String> {
    None
}
pub fn default_port() -> u16 {
    0
}
pub fn default_user() -> String {
    let uid = getuid();
    match User::from_uid(uid) {
        Ok(u) => {
            if let Some(user) = u {
                return user.name;
            }
            "".to_string()
        }
        Err(_) => "".to_string(),
    }
}
pub fn default_password() -> String {
    "".to_string()
}
pub fn default_database() -> String {
    "".to_string()
}
pub fn default_max_connections() -> u32 {
    50
}
pub fn default_min_connections() -> u32 {
    1
}
pub fn default_max_lifetime() -> u64 {
    1800
}
pub fn default_idle_timeout() -> u64 {
    600
}
pub fn default_acquire_timeout() -> u64 {
    10
}
pub fn default_log_enable() -> bool {
    false
}
pub fn default_log_level() -> String {
    "info".to_string()
}
pub fn default_log_slow_level() -> String {
    "danger".to_string()
}
pub fn default_log_threshold() -> u64 {
    1
}
pub fn default_path() -> Option<String> {
    None
}
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

#[derive(Debug, Clone, Default, Serialize)]
pub struct Configuration {
    #[serde(default = "default_socket")]
    pub socket: Option<String>,
    #[serde(default = "default_host")]
    pub host: String,
    #[serde(default = "default_port")]
    pub port: u16,
    #[serde(default = "default_user")]
    pub username: String,
    #[serde(default = "default_password")]
    pub password: String,
    #[serde(default = "default_database")]
    pub database: String,
    #[serde(default = "default_max_connections")]
    pub max_connections: u32,
    #[serde(default = "default_min_connections")]
    pub min_connections: u32,
    #[serde(default = "default_max_lifetime")]
    pub max_lifetime: u64,
    #[serde(default = "default_idle_timeout")]
    pub idle_timeout: u64,
    #[serde(default = "default_acquire_timeout")]
    pub acquire_timeout: u64,
    #[serde(default = "default_log_enable")]
    pub log_enable: bool,
    #[serde(default = "default_log_level")]
    pub log_level: String,
    #[serde(default = "default_log_slow_level")]
    pub log_slow_level: String,
    #[serde(default = "default_log_threshold")]
    pub log_threshold: u64,
    #[serde(flatten)]
    ___flatten: BTreeMap<String, serde_yaml::Value>,
    #[serde(skip)]
    pool_options: OnceLock<PgPoolOptions>,
    #[serde(skip)]
    connect_options: OnceLock<PgConnectOptions>,
    #[serde(skip)]
    pool: OnceLock<ConnectionPool>,
    #[serde(skip)]
    event_manager: Option<Arc<EventManager>>,
}

impl<'de> Deserialize<'de> for Configuration {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        #[derive(Deserialize)]
        struct Shadow {
            #[serde(default = "default_socket")]
            pub socket: Option<String>,
            #[serde(default = "default_host")]
            pub host: String,
            #[serde(default = "default_port")]
            pub port: u16,
            #[serde(default = "default_user")]
            pub username: String,
            #[serde(default = "default_password")]
            pub password: String,
            #[serde(default = "default_database")]
            pub database: String,
            #[serde(default = "default_max_connections")]
            pub max_connections: u32,
            #[serde(default = "default_min_connections")]
            pub min_connections: u32,
            #[serde(default = "default_max_lifetime")]
            pub max_lifetime: u64,
            #[serde(default = "default_idle_timeout")]
            pub idle_timeout: u64,
            #[serde(default = "default_acquire_timeout")]
            pub acquire_timeout: u64,
            #[serde(default = "default_log_enable")]
            pub log_enable: bool,
            #[serde(default = "default_log_level")]
            pub log_level: String,
            #[serde(default = "default_log_slow_level")]
            pub log_slow_level: String,
            #[serde(default = "default_log_threshold")]
            pub log_threshold: u64,
            #[serde(flatten)]
            ___flatten: BTreeMap<String, serde_yaml::Value>,
        }
        let config = Shadow::deserialize(deserializer)?;
        let mut config = Configuration {
            socket: config.socket,
            host: config.host,
            port: config.port,
            username: config.username,
            password: config.password,
            database: config.database,
            max_connections: config.max_connections,
            min_connections: config.min_connections,
            max_lifetime: config.max_lifetime,
            idle_timeout: config.idle_timeout,
            acquire_timeout: config.acquire_timeout,
            log_enable: config.log_enable,
            log_level: config.log_level,
            log_slow_level: config.log_slow_level,
            log_threshold: config.log_threshold,
            ___flatten: config.___flatten,
            pool_options: Default::default(),
            connect_options: Default::default(),
            pool: Default::default(),
            event_manager: None,
        };
        config.__reconfigure();
        Ok(config)
    }
}

impl Configuration {
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
    /// # Example
    ///
    /// ```rust
    
    /// ```
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
        }
        if self.username.trim().is_empty() {
            self.username = default_user();
        }
        // log threshold
        if self.log_threshold < MIN_LOG_THRESHOLD {
            self.log_threshold = MIN_LOG_THRESHOLD;
        } else if self.log_threshold > MAX_LOG_THRESHOLD {
            self.log_threshold = MAX_LOG_THRESHOLD;
        }
        if self.max_connections < MIN_CONNECTIONS {
            self.max_connections = MIN_CONNECTIONS;
        } else if self.max_connections > MAX_CONNECTIONS {
            self.max_connections = MAX_CONNECTIONS;
        }
        if self.min_connections < MIN_CONNECTIONS {
            self.min_connections = MIN_CONNECTIONS;
        } else if self.min_connections > self.max_connections {
            self.min_connections = self.max_connections;
        }
        if self.max_lifetime < MIN_LIFETIME {
            self.max_lifetime = MIN_LIFETIME;
        } else if self.max_lifetime > MAX_LIFETIME {
            self.max_lifetime = MAX_LIFETIME;
        }
        if self.idle_timeout < MIN_IDLE_TIMEOUT {
            self.idle_timeout = MIN_IDLE_TIMEOUT;
        } else if self.idle_timeout > MAX_IDLE_TIMEOUT {
            self.idle_timeout = MAX_IDLE_TIMEOUT;
        }
        if self.acquire_timeout < MIN_ACQUIRE_TIMEOUT {
            self.acquire_timeout = MIN_ACQUIRE_TIMEOUT;
        } else if self.acquire_timeout > MAX_ACQUIRE_TIMEOUT {
            self.acquire_timeout = MAX_ACQUIRE_TIMEOUT;
        }
        if self.idle_timeout > MAX_IDLE_TIMEOUT {
            self.idle_timeout = MAX_IDLE_TIMEOUT;
        } else if self.idle_timeout < MIN_IDLE_TIMEOUT {
            self.idle_timeout = MIN_IDLE_TIMEOUT;
        }
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

    pub fn connect_options(&self) -> &PgConnectOptions {
        self.connect_options.get_or_init(|| {
            PgConnectOptions::new()
                .username(&self.username)
                .password(&self.password)
                .host(&self.host)
                .database(&self.database)
                .options([("timezone", "UTC")])
                .port(self.port)
                .log_slow_statements(self.log_slow_level_filter(), self.log_threshold_duration())
                .log_statements(if self.log_enable {
                    self.log_level_filter()
                } else {
                    LevelFilter::Off
                })
        })
    }

    pub fn pool_options(&self) -> &PgPoolOptions {
        self.pool_options.get_or_init(|| {
            PoolOptions::<Postgres>::new()
                .acquire_timeout(self.acquire_timeout_duration())
                .max_connections(self.max_connections())
                .min_connections(self.min_connections())
                .max_lifetime(self.max_lifetime_duration())
                .idle_timeout(self.idle_timeout_duration())
                .acquire_slow_level(self.log_slow_level_filter())
                .acquire_slow_threshold(self.log_threshold_duration())
        })
    }

    pub fn pool(&self) -> &ConnectionPool {
        self.pool.get_or_init(|| {
            let pool_options = self.pool_options().clone();
            let connect_options = self.connect_options().clone();
            pool_options.connect_lazy_with(connect_options)
        })
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
