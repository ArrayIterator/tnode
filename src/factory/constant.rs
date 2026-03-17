/// The default TCP address and port used by the application.
///
/// This constant specifies the default IP address and port to be used for TCP connections.
///
/// # Value
/// - `127.0.0.1:55155`: Refers to the local loopback address (`127.0.0.1`)
///   and port `55155`.
///
/// # Purpose
/// This constant can be used as a default configuration for establishing
/// TCP connections if no custom address or port is provided.
///
/// # Example
/// ```rust
/// use my_crate::DEFAULT_TCP;
///
/// println!("Default TCP address: {}", DEFAULT_TCP);
/// ```
///
/// In this example, the `DEFAULT_TCP` constant is used to print
/// the default TCP address for the application.
pub const DEFAULT_TCP: &str = "127.0.0.1:55155";

/// A constant defining the minimum number of connections required.
///
/// This constant is used to set a baseline for the minimum number of
/// simultaneous connections that the system, application, or component
/// should maintain. It ensures a predefined level of availability or
/// capacity for handling requests.
///
/// # Value
/// - `128`: The default minimum number of connections.
///
/// # Usage
/// This can be used in scenarios such as
/// - Configuring connection pools.
/// - Establishing limits for network or database connections.
/// - Ensuring adequate resources for concurrent processing.
pub const MIN_CONNECTIONS: usize = 128;

/// A constant representing the maximum number of connections allowed.
///
/// This value is used to limit the number of simultaneous connections
/// that can be handled by the system to ensure stability and resource
/// efficiency.
///
/// # Value
/// - `100_000`: The upper limit for concurrent connections.
///
/// # Usage
/// This constant is typically referenced in scenarios where connection
/// limits are enforced, such as server configuration or connection
/// pool management.
///
/// # Example
/// ```rust
/// if current_connections > MAX_CONNECTIONS {
///     eprintln!("Connection limit exceeded!");
/// }
/// ```
///
/// Adjust this value based on system capacity and performance specifications.
pub const MAX_CONNECTIONS: usize = 65535;

/// The default number of connections allowed.
///
/// `DEFAULT_CONNECTIONS` is a constant that sets the default maximum
/// number of connections that the system or application supports.
/// This value can be used as a baseline for configuring connection
/// limits in networked applications or services.
///
/// # Value
/// - `25_000`: The default maximum number of connections.
///
/// # Usage
/// Use this constant when setting up default configuration parameters
/// to maintain consistency across the application.
///
/// # Example
/// ```rust
/// let max_connections = DEFAULT_CONNECTIONS;
/// println!("The default maximum connections are: {}", max_connections);
/// ```
///
/// # Note
/// You can override this default value in your application configuration
/// if your use case requires a higher or lower connection limit.
pub const DEFAULT_CONNECTIONS: usize = 25_000;

/// A constant that defines the maximum connection rate for the system.
///
/// This value represents the upper limit for the number of connections
/// that can be handled per unit of time. It is used to regulate and ensure
/// the stability and efficiency of the system's connection-handling processes.
///
/// # Value
/// - `4096`: The maximum allowed connection rate.
///
/// # Usage
/// Use this constant to enforce connection rate limits across the system
/// to prevent resource overutilization or denial-of-service scenarios.
///
/// # Example
/// ```rust
/// if current_connection_rate > MAX_CONNECTION_RATE {
///     println!("Connection rate exceeds the maximum allowed rate.");
/// }
/// ```
pub const MAX_CONNECTION_RATE: usize = 65535;

/// A constant representing the minimum allowed connection rate.
///
/// `MIN_CONNECTION_RATE` defines the minimum number of connections
/// per second that the system or application can support. This value
/// is used to enforce rate-limiting policies or to ensure the system
/// operates within acceptable performance thresholds.
///
/// # Value
/// - `32`: The minimum connection rate.
///
/// # Usage
/// Ensure any connection handling logic respects this limit to prevent
/// overloading the system.
///
pub const MIN_CONNECTION_RATE: usize = 32;

/// The default connection rate limit.
///
/// `DEFAULT_CONNECTION_RATE` defines the maximum number of connection attempts
/// allowed per second. This constant is typically used to prevent overwhelming
/// a network service with excessive connection requests and to ensure fair
/// usage of resources.
///
/// The value is set to `256` connection attempts per second by default, but it
/// can be adjusted as necessary based on system requirements or performance constraints.
///
/// # Example
///
/// ```rust
/// use your_module::DEFAULT_CONNECTION_RATE;
///
/// fn main() {
///     println!("The default connection rate is: {}", DEFAULT_CONNECTION_RATE);
/// }
/// ```
///
/// This constant is of type `usize`.
pub const DEFAULT_CONNECTION_RATE: usize = 256;

/// The maximum number of pending connections that can be queued for a server.
///
/// This constant defines the upper limit for the backlog of unaccepted incoming
/// connections in a server's listener queue. When the queue is full, additional
/// connection attempts maybe rejected by the operating system.
///
/// # Value
/// - `65535` is the predefined maximum backlog size.
/// - `8192` is the recommendation maximum backlog size.
///
/// # Usage
/// This value can be used when configuring the backlog size in server applications
/// to ensure that the application does not set a value that exceeds system or
/// application limits.
///
/// # Example
/// ```rust
/// use my_crate::MAX_BACKLOG;
///
/// let backlog_size = MAX_BACKLOG; // Use the predefined maximum
/// ```
///
/// Note: The actual maximum backlog supported depends on the operating system.
pub const MAX_BACKLOG: u32 = 65535;

/// The minimum number of pending connection requests that can be
/// queued for a listening socket. This constant is typically used
/// when configuring the backlog parameter for socket listeners.
///
/// # Value
/// - `128`: Represents the default minimum backlog size deemed enough
///   for handling incoming connection requests in typical use cases.
///
/// # Usage
/// This constant can be used when setting up a socket listener to
/// ensure there is at least the specified number of connection requests
/// allowed in the pending queue.
///
/// # Example
/// ```rust
/// use your_module::MIN_BACKLOG;
///
/// let backlog = MIN_BACKLOG; // Use the constant in your listener configuration
/// ```
pub const MIN_BACKLOG: u32 = 128;

/// A constant representing the default backlog size for a queue.
///
/// This value is typically used to set the maximum number of pending
/// connections in a queue when creating a network listener (e.g.,
/// a TCP listener). The default value is set to `4096`.
///
/// # Examples
///
/// ```
/// use your_crate_name::DEFAULT_BACKLOG;
///
/// println!("Default backlog: {}", DEFAULT_BACKLOG);
/// ```
///
/// # Note
///
/// Adjust this value based on your application's requirements and
/// system limitations.
pub const DEFAULT_BACKLOG: u32 = 4096;

/// A constant representing the minimum allowable value for the "RLIMIT_NOFILE" resource limit.
///
/// `RLIMIT_NOFILE` specifies the maximum number of file descriptors that a process can open
/// concurrently. This constant defines a sensible default lower bound for the limit to ensure
/// sufficient resources are available for typical workloads.
///
/// # Value
/// The constant is set to `4096`, which provides a reasonable minimum threshold for applications.
///
/// # Usage
/// This constant can be used to configure or validate resource limits when working with system-level
/// configurations or libraries that interact with file descriptor limits.
///
/// # Example
/// ```rust
/// use std::cmp;
///
/// let default_rlimit = 65536; // Example value for maximum file descriptors
/// let applied_rlimit = cmp::max(default_rlimit, MIN_RLIMIT_NOFILE as u64);
/// println!("Applied RLIMIT_NOFILE: {}", applied_rlimit);
/// ```
///
/// # Platform Compatibility
/// Ensure that you test your configurations on target platforms, as specific limits
/// may vary between operating systems.
pub const MIN_RLIMIT_NOFILE: u64 = 4096;

/// The maximum value for `RLIMIT_NOFILE`, which defines the upper limit on the number of open file descriptors
/// that a process can have. This constant is set to `1048576`.
///
/// # Platform-specific Behavior
/// - This value is typically used in Unix-like operating systems where `RLIMIT_NOFILE` is used to specify and
///   query the resource limit for open file descriptors.
///
/// # Usage
/// This constant can be used to configure or check maximum file descriptor limits programmatically when working
/// with system resources that depend on file descriptors, such as sockets, files, or pipes.
///
/// # Example
/// ```rust
/// use std::io;
///
/// fn check_max_limit() -> u64 {
///     MAX_RLIMIT_NOFILE
/// }
///
/// println!("The max RLIMIT_NOFILE is: {}", check_max_limit());
/// ```
pub const MAX_RLIMIT_NOFILE: u64 = 1048576;

/// `DEFAULT_RLIMIT_NOFILE` is a constant that specifies the default maximum
/// number of file descriptors that a process can open at once.
///
/// This value is commonly used to set or reference the `RLIMIT_NOFILE` limit in
/// operating system configurations or resource management scenarios.
///
/// The value is set to `65535`, which is typically enough for most
/// applications but can be adjusted depending on system requirements.
///
/// # Usage
/// This constant can be used when setting resource limits or when a specific
/// maximum file descriptor limit is required.
///
/// # Example
/// ```
/// use your_crate::DEFAULT_RLIMIT_NOFILE;
///
/// fn print_file_limit() {
///     println!("The default maximum open file limit is: {}", DEFAULT_RLIMIT_NOFILE);
/// }
/// ```
pub const DEFAULT_RLIMIT_NOFILE: u64 = 65535;

/// A constant representing the maximum allowed operation timeout in seconds.
///
/// This constant is used to specify the upper limit for any operation's timeout duration.
/// The value is set to 3600 seconds, which is equivalent to 1 hour. Operations exceeding
/// this limit will timeout to ensure the system's responsiveness and stability.
///
/// # Example
///
/// ```rust
/// if operation_duration > MAX_OPERATION_TIMEOUT {
///     println!("Operation timed out.");
/// }
/// ```
///
/// # Notes
/// - This value should not be modified unless there is a specific requirement to do so.
/// - Ensure that any changes to this constant are tested for compatibility with dependent systems.
pub const MAX_OPERATION_TIMEOUT: usize = 3600;

/// A constant representing the minimum allowed operation timeout.
///
/// This value is used to ensure that no operation has a timeout
/// less than the defined minimum, measured in seconds.
///
/// # Example
/// ```rust
/// if timeout < MIN_OPERATION_TIMEOUT {
///     panic!("Timeout must not be less than {}", MIN_OPERATION_TIMEOUT);
/// }
/// ```
///
/// # Value
/// - `1` (one second)
pub const MIN_OPERATION_TIMEOUT: usize = 1;

/// The default timeout duration (in seconds) for operations.
///
/// This constant is used to define the maximum amount of time an operation
/// should take before being considered as timed out. It applies to
/// scenarios where the user provides no custom timeout value.
///
/// # Example
/// ```
/// use your_module::DEFAULT_OPERATION_TIMEOUT;
///
/// println!("Default timeout is {} seconds.", DEFAULT_OPERATION_TIMEOUT);
/// ```
///
/// Default value: `10` seconds.
pub const DEFAULT_OPERATION_TIMEOUT: usize = 10;

pub const MIN_REQUEST_TIMEOUT: usize = 0; // 0 mean disable
pub const MAX_REQUEST_TIMEOUT: usize = 3600;
pub const DEFAULT_REQUEST_TIMEOUT: usize = 10;

pub const MIN_DISCONNECT_TIMEOUT: usize = 0;
pub const MAX_DISCONNECT_TIMEOUT: usize = 3600;
pub const DEFAULT_DISCONNECT_TIMEOUT: usize = 10;

pub const DEFAULT_KEEP_ALIVE: usize = 75;

pub const MIN_KEEP_ALIVE: usize = 0;

pub const MAX_KEEP_ALIVE: usize = 43200;
pub const MIN_SSL_SESSION_CACHE: usize = 2;
pub const MAX_SSL_SESSION_CACHE: usize = 256;
pub const DEFAULT_SSL_SESSION_CACHE: usize = 32;
pub const DEFAULT_SESSION_NAME: &str = "vault";
pub const DEFAULT_CSRF_DURATION_MINUTES: usize = 30;
pub const MAX_CSRF_DURATION_MINUTES: usize = 3600 * 24; // max 1 day
pub const MIN_CSRF_DURATION_MINUTES: usize = 5; // min 5 minutes
pub const DEFAULT_AUTO_CLEAN_MEMORY_SIZE_BYTES: usize = 384 * 1024 * 1024;
pub const MIN_AUTO_CLEAN_MEMORY_SIZE_BYTES: usize = 128 * 1024 * 1024;

// auto clean memory interval in seconds
pub const DEFAULT_AUTO_CLEAN_MEMORY_INTERVAL: usize = 3600;
pub const MAX_AUTO_CLEAN_MEMORY_INTERVAL: usize = 2592000; // 1 month = 30 days * 24 hours * 60 minutes
pub const MIN_AUTO_CLEAN_MEMORY_INTERVAL: usize = 30; // max 1 week
pub const DEFAULT_DASHBOARD_PATH: &str = "dashboard";

/// ```
/// The `DEFAULT_CONFIG` constant contains the default configuration settings for the application in YAML format.
/// This configuration defines parameters for the application setup, such as application mode, storage path,
/// TCP and Unix socket settings, and operational timeouts, as well as database connection and behavior.
///
pub const DEFAULT_CONFIG: &str = // language=yaml
    r#"# ----------------------------------------
# BEGIN CONFIGURATION
# ----------------------------------------
#
# ----------------------------------------
# Application configuration
app:
  # Application mode
  # development or production
  # Default: production
  mode: "production"
  # Dashboard path
  # Default Dashboard
  # Allowed characters [a-z0-9]([a-z0-9_-]?[a-z0-9])? case insensitive
  dashboard_path: "dashboard"
  # Session name
  #  used to store session id in cookie
  # session name must be ([a-zA-Z0-9_-]+)
  # default: vault
  session_name: vault
  # CSRF token duration in minutes
  # minimum: 5 minutes, maximum: 86400 (1 day)
  # default: 30 minutes
  csrf_durations: 30
  # Secret key used to encrypt the sensitive content / data
  # default: (empty)
  secret_key: "change-this-to-a-random-secret-string"
  # Salt key used to encrypt sensitive data
  # default: (empty)
  salt_key: "change-this-to-a-random-salt-string"
  # Default user run as (affected on root permissions)
  # default: (empty) auto-detect binary owner
  user: ""
  # List of TCP Listen addresses
  # used for multiple binding
  # only localhost is supported for non-IP addresses
  # Default: [127.0.0.1:55155]
  tcp:
    - "127.0.0.1:55080"
  # SSL Configuration
  # 443 always
  ssl:
    listen:
      - "127.0.0.1:55443"
    cert: ""
    key: ""
    # SSL Session cache size in MB
    # minimum: 2, maximum: 256
    # default : 32
    ssl_session_cache: 32
  # Max worker
  # default: 0 (auto detect - use available cores)
  workers: 0
  # Operation timeout in seconds for non websocket operations
  # minimum: 1, maximum: 3600
  # Default: 10
  operation_timeout: 10
  # Request timeout in seconds
  # minimum: 0, maximum: 3600
  # Default: 10
  # 0 mean disable
  request_timeout: 10
  # Disconnect timeout in seconds
  # minimum: 0, maximum: 3600
  # Default: 5
  disconnect_timeout: 5
  # Maximum number of concurrent connections
  # maximum: 65535, minimum: 128
  # Default: 25K / 25000
  max_connections: 25000
  # Sets the per-worker maximum concurrent TLS connection limit.
  # All listeners will stop accepting connections when this limit is reached. It can be used to limit the global TLS CPU usage.
  # minimum: 32, maximum: 65535
  # Default: 0 (auto calculation)
  max_connections_rate: 0
  # Backlog size for the server socket
  # maximum: 65535, minimum: 128 (maximum recommended is: 8192)
  # Default: 4096
  backlog: 4096
  # Rlimit nofile
  # The maximum number of open file descriptors that the process can have.
  # maximum: 1048576, minimum: 4096
  # Default: 65535
  rlimit_nofile: 65535
  # Keep-alive connection
  # minimum: 0, maximum: 7200
  # Default: 75
  # 0 means disabled
  keep_alive: 75
  # Auto clean memory
  # Default: true
  auto_clean_memory: true
  # Auto clean memory threshold
  # minimum: 128MB
  # Default: 384MB
  # Without unit it will be used bytes as unit
  auto_clean_memory_size: "384M"
  # Auto clean memory interval in seconds
  # If you need to use this feature, recommended using every 5 minutes (300)
  # minimum: 30 (30 seconds), maximum: 1 month = 30 days * 24 hours * 60 minutes
  # Default: 1H
  # Without unit it will be used second as unit
  # Uppercase M as Month, as lowercase m as Minute
  auto_clean_memory_interval: "1H"
# ----------------------------------------
# Redis configuration
# Use Redis
redis:
  # Unix socket path
  # Default: (empty)
  unix_socket: ""
  # Redis host
  # Default: localhost
  host: localhost
  # Redis port
  # Default: 6379
  port: 6379
  # Redis username
  # Default: (empty)
  username: ""
  # Redis password
  # Default: (empty)
  password: ""
  # Redis database
  # minimum: 0, maximum: 16
  # Default: 0
  database: 0
  # Maximum pools
  # minimum: 1, maximum: 128
  # Default: 10
  max_pools: 10
  # Minimum number of idle connections in the pool
  # minimum: 1, maximum: 64
  # Default: 1
  minimum_idle_connections: 1
  # Idle connection timeout in seconds
  # minimum: 1, maximum: 86400 (1 day)
  # Default: 600
  idle_timeout: 600
  # Connection timeout in seconds
  # minimum: 1, maximum: 3600
  # Default: 5
  connect_timeout: 5
# ----------------------------------------
# Database configuration
# Database connection pool settings
# Use PostgreSQL
database:
  # Database driver
  # postgresql, sqlite, mysql
  driver: "postgresql"
  # Unix socket path
  # Default: (empty)
  socket: null
  # Database host
  # Default: localhost
  host: localhost
  # Database port
  # Default: 5432
  port: 5432
  # Database username
  # Default: (empty)
  username: ""
  # Database password
  password: ""
  # Database name
  # Default: (empty)
  database: ""
  # Maximum number of connections in the pool
  # Default: 50
  max_connections: 50
  # Minimum number of idle connections in the pool
  # Default: 1
  min_connections: 1
  # Connection acquisition timeout in seconds
  # Default: 1800
  max_lifetime: 1800
  # Statement timeout in seconds
  # Default: 5
  statement_timeout: 5
  # Idle connection timeout in seconds
  # Default: 600
  idle_timeout: 600
  # Connection timeout in seconds
  # Default: 10
  acquire_timeout: 10
  # Enable logging
  # Default: false
  log_enable: false
  # Log level
  # debug, info, warn, error, off
  # Default: Warn
  log_level: "warn"
  # Log slow queries
  # debug, info, warn, error, off
  # Default: error
  log_slow_level: "error"
  # Log threshold in seconds
  # Default: 1
  log_threshold: 1
  # Collation (only for MySQL)
  # Default: utf8mb4_general_ci
  collation: "utf8mb4_general_ci"
# ----------------------------------------
# Licenses / API Key based on requirements
#
licenses:
  # maxmind geo api key
  maxmind: ""
  # put any other license configuration here

# ----------------------------------------
# END
# ----------------------------------------
"#;
