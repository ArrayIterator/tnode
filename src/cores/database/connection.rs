use crate::cores::database::adapter::Adapter;
use crate::cores::database::configuration::Configuration;
use crate::cores::database::entity::Entity;
use crate::cores::database::query_builder::QueryBuilder;
use crate::cores::system::event_manager::EventManager;
use sqlx::{Database, Pool, Any};
use std::fmt::Debug;
use std::ops::Deref;
use std::sync::{Arc, OnceLock};

pub type DbType = Any;
pub type ConnectionPool = Pool<DbType>;

#[derive(Debug)]
pub struct Connection {
    config: Arc<Configuration>,
    pool: Arc<OnceLock<Pool<DbType>>>,
    event_manager: Option<Arc<EventManager>>,
}

/// # Examples
///
/// ```rust
/// # use std::sync::Arc;
/// # // You may also need to import your crate's types here for the example to work
/// # use your_crate_name::cores::database::configuration::Configuration;
/// # use your_crate_name::Connection;
///
/// let config = Configuration {
///     driver: Driver::Postgresql,
///     // ... fields
/// };
///
/// // If Connection::from(config) is used:
/// let connection = Connection::from(config);
/// ```
impl Connection {
    /// Creates a `Connection` instance from the given configuration.
    ///
    /// # Arguments
    ///
    /// * `config` - A `Config` instance containing the connection settings required to establish the connection.
    ///              This includes details such as the database driver, credentials, connection timeouts, and more.
    ///
    /// # Returns
    ///
    /// * `Result<Connection, Error>` - Returns a `Connection` instance upon success, or an `Error` if the connection
    ///                                  configuration or initiation fails.
    ///
    /// # Behavior
    ///
    /// This function constructs a connection pool and configures it based on the provided `Config` options:
    /// - Timeout duration for acquiring a connection.
    /// - Maximum and minimum number of connections in the pool.
    /// - Lifetime and idle time settings for the connections.
    /// - Logging settings for slow acquisition and query execution.
    ///
    /// Depending on the specified database driver (`Postgresql`, `Sqlite`, or `Mysql`), it prepares the appropriate
    /// connection options:
    ///
    /// ## For Postgres:
    /// - Sets the USER credentials, host, database name, and timezone.
    /// - Configures socket options if specified or falls back to the URL if sockets are not configured.
    ///
    /// The method applies logging configurations that can be enabled or disabled using the `log_enable` option. It also
    /// respects the specified logging filter levels and thresholds for logging slow queries/statements.
    ///
    /// Finally, it establishes the connection pool lazily using the configured options and returns the `Connection` instance.
    ///
    /// # Errors
    ///
    /// Returns an error if:
    /// - The provided configuration is invalid.
    /// - The connection details (e.g., credentials, database URLs) are incorrect.
    /// - An issue arises during the initialization of the connection pool.
    ///
    /// # Examples
    ///
    /// ```rust
    /// let config = Config {
    ///     driver: Driver::Postgresql,
    ///     host: "localhost".to_string(),
    ///     username: "USER".to_string(),
    ///     password: "password".to_string(),
    ///     database: "example_db".to_string(),
    ///     port: 5432,
    ///     ..Default::default()
    /// };
    ///
    /// let connection = Connection::from(config);
    /// match connection {
    ///     Ok(conn) => println!("Connection established successfully."),
    ///     Err(e) => eprintln!("Error establishing connection: {:?}", e),
    /// }
    /// ```
    pub fn new(config: Arc<Configuration>, event_manager: Option<Arc<EventManager>>) -> Self {
        Self {
            config,
            pool: Default::default(),
            event_manager,
        }
    }

    pub fn with_event_manager(config: Arc<Configuration>, event: Arc<EventManager>) -> Self {
        Self::new(config, Some(event))
    }

    pub fn set_event_manager(&mut self, event_manager: Option<Arc<EventManager>>) {
        self.event_manager = event_manager
    }

    /// Retrieves a reference to the current `Configuration` of the instance.
    ///
    /// # Returns
    /// A reference to the `Configuration` object stored within the instance.
    ///
    /// # Example
    /// ```
    /// let config = instance.get_config();
    /// ```
    /// This method is useful for accessing the configuration details of the instance without
    /// taking ownership or modifying it.
    pub fn get_config(&self) -> &Configuration {
        &self.config
    }

    /// Provides access to the database connection pool.
    ///
    /// # Returns
    ///
    /// A reference to the `Pool<Any>` instance representing the pool of database connections.
    ///
    /// # Usage
    ///
    /// This method allows you to retrieve the database connection pool associated with the current context.
    /// It can be used to manage and execute database queries by obtaining connections from the pool.
    ///
    /// # Example
    ///
    /// ```rust
    /// let pool = my_instance.pool();
    /// let conn = pool.get().expect("Failed to get a connection from the pool");
    /// // Use the connection for database operations
    /// ```
    ///
    /// # Notes
    ///
    /// - Ensure proper error handling when retrieving connections from the pool.
    /// - Avoid exhausting the pool by releasing connections back after use, typically via RAII or explicit release mechanisms.
    pub fn pool(&self) -> &Pool<DbType> {
        &self.pool.get_or_init(|| DbType::pool(&self.config))
    }
    /// Creates a new `QueryBuilder` instance for constructing database queries.
    ///
    /// This function is generic over the entity type `E` that implements the `Entity` trait
    /// for a specific database `D`. It leverages the connection pool associated with `self`
    /// to initialize the query builder.
    ///
    /// # Type Parameters
    /// - `E`: An entity type that implements the `Entity` trait for database `D`.
    ///
    /// # Returns
    /// A `QueryBuilder<'_, E>` instance tied to the connection pool of the struct.
    ///
    /// # Example
    /// ```
    /// let query_builder = my_struct.create_query_builder::<MyEntity>();
    /// // Use `query_builder` to construct a query.
    /// ```
    pub fn create_query_builder<E: Entity>(&self) -> QueryBuilder<'_, E>
    where
        for<'q> <DbType as Database>::Arguments<'q>: sqlx::IntoArguments<'q, DbType>,
        for<'c> &'c Pool<DbType>: sqlx::Executor<'c, Database = DbType>,
    {
        QueryBuilder::new(self.pool())
    }
}

impl Deref for Connection {
    type Target = Pool<DbType>;
    fn deref(&self) -> &Self::Target {
        self.pool()
    }
}

impl From<Connection> for Pool<DbType> {
    fn from(conn: Connection) -> Self {
        conn.pool().clone()
    }
}

impl From<&Connection> for Pool<DbType> {
    fn from(conn: &Connection) -> Self {
        conn.pool().clone()
    }
}

impl From<Configuration> for Connection {
    fn from(config: Configuration) -> Self {
        let em = config.event_manager();
        Self::new(Arc::new(config), em)
    }
}

impl From<Arc<Configuration>> for Connection {
    fn from(config: Arc<Configuration>) -> Self {
        let em = config.event_manager();
        Self::new(config, em)
    }
}

impl From<&Arc<Configuration>> for Connection {
    fn from(config: &Arc<Configuration>) -> Self {
        Self::new(config.clone(), config.event_manager())
    }
}
