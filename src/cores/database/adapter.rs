use crate::cores::database::configuration::Configuration;
use log::LevelFilter;
use serde::{Deserialize, Serialize};
use sqlx::{
    Any, ConnectOptions, Database, MySql, Pool, Postgres, Sqlite,
    any::{AnyConnectOptions, AnyPoolOptions},
    mysql::{MySqlConnectOptions, MySqlPoolOptions},
    pool::PoolOptions,
    postgres::{PgConnectOptions, PgPoolOptions},
    sqlite::{SqliteConnectOptions, SqlitePoolOptions},
};
use std::{ops::Deref, str::FromStr, sync::Arc};

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
pub enum Driver {
    Postgresql = 0,
    MySql = 1,
    Sqlite = 2,
}

pub trait WrappedConnectOptions: Send + Sync + 'static {
    type Wrapped: ConnectOptions + Send + Sync + 'static;
    fn wrapped(&self) -> Arc<Self::Wrapped>;
}

#[derive(Debug, Clone)]
struct ObjectConnectOptions<T>
where
    T: Send + Sync + 'static,
{
    wrapped: Arc<T>,
}

impl<T: ConnectOptions + Send + Sync + 'static> FromStr for ObjectConnectOptions<T> {
    type Err = sqlx::Error;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        let url = url::Url::parse(s).map_err(|e| sqlx::Error::Configuration(Box::new(e)))?;
        let options = T::from_url(&url)?;
        Ok(ObjectConnectOptions {
            wrapped: Arc::new(options),
        })
    }
}

impl<T: ConnectOptions + Send + Sync + 'static> WrappedConnectOptions for T {
    type Wrapped = T;
    fn wrapped(&self) -> Arc<T> {
        Arc::new(self.clone())
    }
}

impl<T: ConnectOptions + Send + Sync + 'static> From<T> for ObjectConnectOptions<T> {
    fn from(options: T) -> Self {
        ObjectConnectOptions {
            wrapped: Arc::new(options),
        }
    }
}

impl<T: ConnectOptions + Send + Sync + 'static> ObjectConnectOptions<T> {
    pub fn new(options: T) -> Self {
        Self::from(options)
    }
}

impl<T: ConnectOptions + Send + Sync + 'static> Deref for ObjectConnectOptions<T> {
    type Target = T;
    fn deref(&self) -> &Self::Target {
        &self.wrapped
    }
}

impl<T: ConnectOptions + Send + Sync + 'static> WrappedConnectOptions for ObjectConnectOptions<T> {
    type Wrapped = T;

    fn wrapped(&self) -> Arc<Self::Wrapped> {
        Arc::clone(&self.wrapped)
    }
}

pub trait Adapter {
    type DbType: Database + Send + Sync + 'static;
    type ConnectOptions: ConnectOptions + Send + Sync + 'static;
    fn pool_options(config: &Configuration) -> PoolOptions<Self::DbType>
    where
        Self: Sized;
    fn connect_options(config: &Configuration) -> Self::ConnectOptions
    where
        Self: Sized;
    fn pool(config: &Configuration) -> Pool<Self::DbType>
    where
        Self: Sized;
}

impl Adapter for Postgres {
    type DbType = Postgres;
    type ConnectOptions = PgConnectOptions;
    fn pool_options(config: &Configuration) -> PoolOptions<Postgres> {
        PgPoolOptions::new()
            .acquire_timeout(config.acquire_timeout_duration())
            .max_connections(config.max_connections())
            .min_connections(config.min_connections())
            .max_lifetime(config.max_lifetime_duration())
            .idle_timeout(config.idle_timeout_duration())
            .acquire_slow_level(config.log_slow_level_filter())
            .acquire_slow_threshold(config.log_threshold_duration())
    }
    fn connect_options(config: &Configuration) -> Self::ConnectOptions {
        PgConnectOptions::new()
            .username(&config.username())
            .password(&config.password())
            .host(&config.host())
            .database(&config.database())
            .port(config.port())
            .options([
                ("timezone", "UTC"),         // Set timezone to UTC
                ("client_encoding", "UTF8"), // Set client encoding to UTF-8
                (
                    "statement_timeout",
                    &(config.statement_timeout() * 1000).to_string(),
                ), // Set statement timeout in milliseconds
            ])
            .log_slow_statements(
                config.log_slow_level_filter(),
                config.log_threshold_duration(),
            )
            .log_statements(if config.log_enable() {
                config.log_level_filter()
            } else {
                LevelFilter::Off
            })
    }
    fn pool(config: &Configuration) -> Pool<Self::DbType>
    where
        Self: Sized,
    {
        Pool::connect_lazy_with(Self::connect_options(config))
    }
}

impl Adapter for Sqlite {
    type DbType = Sqlite;
    type ConnectOptions = SqliteConnectOptions;
    fn pool_options(config: &Configuration) -> PoolOptions<Self::DbType> {
        SqlitePoolOptions::new()
            .acquire_timeout(config.acquire_timeout_duration())
            .max_connections(config.max_connections())
            .min_connections(config.min_connections())
            .max_lifetime(config.max_lifetime_duration())
            .idle_timeout(config.idle_timeout_duration())
            .acquire_slow_level(config.log_slow_level_filter())
            .acquire_slow_threshold(config.log_threshold_duration())
    }
    fn connect_options(config: &Configuration) -> Self::ConnectOptions {
        SqliteConnectOptions::new()
            .filename(&config.database())
            .log_slow_statements(
                config.log_slow_level_filter(),
                config.log_threshold_duration(),
            )
            .log_statements(if config.log_enable() {
                config.log_level_filter()
            } else {
                LevelFilter::Off
            })
    }

    fn pool(config: &Configuration) -> Pool<Self::DbType>
    where
        Self: Sized,
    {
        Pool::connect_lazy_with(Self::connect_options(config))
    }
}

impl Adapter for MySql {
    type DbType = MySql;
    type ConnectOptions = MySqlConnectOptions;

    fn pool_options(config: &Configuration) -> PoolOptions<Self::DbType> {
        MySqlPoolOptions::new()
            .acquire_timeout(config.acquire_timeout_duration())
            .max_connections(config.max_connections())
            .min_connections(config.min_connections())
            .max_lifetime(config.max_lifetime_duration())
            .idle_timeout(config.idle_timeout_duration())
            .acquire_slow_level(config.log_slow_level_filter())
            .acquire_slow_threshold(config.log_threshold_duration())
    }

    fn connect_options(config: &Configuration) -> Self::ConnectOptions {
        MySqlConnectOptions::new()
            .username(&config.username())
            .password(&config.password())
            .host(&config.host())
            .database(&config.database())
            .port(config.port())
            .charset("utf8mb4") // Set character set to UTF-8
            .collation(&config.collation()) // Set collation based on configuration
            .timezone(Some("+00:00".to_string())) // Set timezone to UTC
            .log_slow_statements(
                config.log_slow_level_filter(),
                config.log_threshold_duration(),
            )
            .log_statements(if config.log_enable() {
                config.log_level_filter()
            } else {
                LevelFilter::Off
            })
    }
    fn pool(config: &Configuration) -> Pool<Self::DbType>
    where
        Self: Sized,
    {
        Pool::connect_lazy_with(Self::connect_options(config))
    }
}

impl Adapter for Any {
    type DbType = Any;
    type ConnectOptions = AnyConnectOptions;

    fn pool_options(config: &Configuration) -> PoolOptions<Self::DbType> {
        AnyPoolOptions::new()
            .acquire_timeout(config.acquire_timeout_duration())
            .max_connections(config.max_connections())
            .min_connections(config.min_connections())
            .max_lifetime(config.max_lifetime_duration())
            .idle_timeout(config.idle_timeout_duration())
            .acquire_slow_level(config.log_slow_level_filter())
            .acquire_slow_threshold(config.log_threshold_duration())
    }
    fn connect_options(config: &Configuration) -> Self::ConnectOptions {
        let url = match config.driver() {
            Driver::Postgresql => Postgres::connect_options(config).to_url_lossy(),
            Driver::MySql => MySql::connect_options(config).to_url_lossy(),
            Driver::Sqlite => Sqlite::connect_options(config).to_url_lossy(),
        };
        let mut any = AnyConnectOptions::from_url(&url).unwrap();
        any.log_settings.slow_statements_level = config.log_slow_level_filter();
        any.log_settings.slow_statements_duration = config.log_threshold_duration();
        any.log_settings.statements_level = if config.log_enable() {
            config.log_level_filter()
        } else {
            LevelFilter::Off
        };
        any
    }
    fn pool(config: &Configuration) -> Pool<Self::DbType>
    where
        Self: Sized,
    {
        Pool::connect_lazy_with(Self::connect_options(config))
    }
}

impl Driver {
    pub fn name(&self) -> &'static str {
        match self {
            Driver::Postgresql => Postgres::NAME,
            Driver::MySql => MySql::NAME,
            Driver::Sqlite => Sqlite::NAME,
        }
    }
}

pub struct DbPool<T: Database> {
    pool: Arc<Pool<T>>,
}

impl<T: Database> DbPool<T> {
    pub fn new(pool: Pool<T>) -> Self {
        DbPool {
            pool: Arc::new(pool),
        }
    }
    pub fn get_pool(&self) -> Arc<Pool<T>> {
        self.pool.clone()
    }
    pub fn pool(&self) -> &Pool<T> {
        &self.pool.as_ref()
    }
}
