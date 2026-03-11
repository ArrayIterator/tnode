use crate::cores::system::error::{Error, ResultError};
use parking_lot::Mutex;
use r2d2::{Pool, PooledConnection};
use redis::{Client, ConnectionInfo, IntoConnectionInfo, RedisResult};
use scheduled_thread_pool::ScheduledThreadPool;
use serde::{Deserialize, Serialize};
use std::fmt::Debug;
use std::sync::Arc;
use std::time::Duration;

pub const MIN_REDIS_DB: usize = 0;
pub const MAX_REDIS_DB: usize = 16;
pub const DEFAULT_REDIS_DB: usize = 0;
pub const DEFAULT_REDIS_HOST: &str = "localhost";
pub const DEFAULT_REDIS_PORT: u16 = 6379;
pub const DEFAULT_REDIS_USERNAME: &str = "";
pub const DEFAULT_REDIS_PASSWORD: &str = "";
pub const DEFAULT_REDIS_CONNECT_TIMEOUT: usize = 5;
pub const MIN_REDIS_CONNECT_TIMEOUT: usize = 1;
pub const MAX_REDIS_CONNECT_TIMEOUT: usize = 30;
pub const DEFAULT_REDIS_IDLE_TIMEOUT: usize = 600;
pub const MIN_REDIS_IDLE_TIMEOUT: usize = 1;
pub const MAX_REDIS_IDLE_TIMEOUT: usize = 86400; // 1 day
pub const DEFAULT_REDIS_POOL: usize = 10;
pub const MIN_REDIS_POOL: usize = 1;
pub const MAX_REDIS_POOL: usize = 128;
pub const DEFAULT_REDIS_MIN_IDLE_CONNECTIONS: usize = 1;
pub const MIN_REDIS_MIN_IDLE_CONNECTIONS: usize = 1;
pub const MAX_REDIS_MIN_IDLE_CONNECTIONS: usize = 64;

#[derive(Debug, Clone, Serialize)]
pub struct RedisConfig {
    #[serde(default)]
    pub host: String,
    #[serde(default)]
    pub unix_socket: String,
    #[serde(default)]
    pub port: u16,
    #[serde(default)]
    pub username: String,
    #[serde(default)]
    pub password: String,
    #[serde(default)]
    pub database: usize,
    #[serde(default)]
    pub max_pools: usize,
    #[serde(default)]
    pub minimum_idle_connections: usize,
    #[serde(default)]
    pub idle_timeout: usize,
    #[serde(default)]
    pub connect_timeout: usize,
    #[serde(skip)]
    __reconfigured: bool,
}

impl RedisConfig {
    fn __reconfigure(&mut self) {
        if self.__reconfigured {
            return;
        }
        if self.host.is_empty() {
            self.host = DEFAULT_REDIS_HOST.to_string();
        }
        if self.port == 0 {
            self.port = DEFAULT_REDIS_PORT;
        }
        if self.username.is_empty() {
            self.username = DEFAULT_REDIS_USERNAME.to_string();
        }
        if self.password.is_empty() {
            self.password = DEFAULT_REDIS_PASSWORD.to_string();
        }
        if self.database > MAX_REDIS_DB {
            self.database = MAX_REDIS_DB;
        } else if self.database < MIN_REDIS_DB {
            self.database = MIN_REDIS_DB;
        }
        if self.max_pools > MAX_REDIS_POOL {
            self.max_pools = MAX_REDIS_POOL;
        } else if self.max_pools < MIN_REDIS_POOL {
            self.max_pools = MIN_REDIS_POOL;
        }
        if self.minimum_idle_connections > MAX_REDIS_MIN_IDLE_CONNECTIONS {
            self.minimum_idle_connections = MAX_REDIS_MIN_IDLE_CONNECTIONS;
        } else if self.minimum_idle_connections > MAX_REDIS_MIN_IDLE_CONNECTIONS {
            self.minimum_idle_connections = MAX_REDIS_MIN_IDLE_CONNECTIONS;
        }
        if self.idle_timeout > MAX_REDIS_IDLE_TIMEOUT {
            self.idle_timeout = MAX_REDIS_IDLE_TIMEOUT;
        } else if self.idle_timeout < MIN_REDIS_IDLE_TIMEOUT {
            self.idle_timeout = MIN_REDIS_IDLE_TIMEOUT;
        }
    }

    pub fn to_connection_info(&self) -> RedisResult<ConnectionInfo> {
        let db = self.database;
        let mut url = String::new();
        if !self.unix_socket.is_empty() {
            url.push_str(&format!("unix://{}", self.unix_socket));
        } else {
            if !self.username.is_empty() {
                url.push_str(&format!("@{}:{}", self.username, self.password));
            } else if !self.password.is_empty() {
                url.push_str(&format!(":{}", self.password));
            } else {
                url.push_str(&format!("@{}:{}", self.host, self.port));
            }
            url.push_str(&format!("/{}", db));
        }
        url.into_connection_info()
    }
}

impl Default for RedisConfig {
    fn default() -> Self {
        Self {
            unix_socket: "".to_string(),
            host: DEFAULT_REDIS_HOST.to_string(),
            port: DEFAULT_REDIS_PORT,
            username: DEFAULT_REDIS_USERNAME.to_string(),
            password: DEFAULT_REDIS_PASSWORD.to_string(),
            database: DEFAULT_REDIS_DB,
            max_pools: DEFAULT_REDIS_POOL,
            minimum_idle_connections: DEFAULT_REDIS_MIN_IDLE_CONNECTIONS,
            idle_timeout: DEFAULT_REDIS_IDLE_TIMEOUT,
            connect_timeout: DEFAULT_REDIS_CONNECT_TIMEOUT,
            __reconfigured: false,
        }
    }
}

impl<'de> Deserialize<'de> for RedisConfig {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        //noinspection DuplicatedCode
        #[derive(Deserialize)]
        struct Shadow {
            #[serde(default)]
            pub host: String,
            #[serde(default)]
            pub unix_socket: String,
            #[serde(default)]
            pub port: u16,
            #[serde(default)]
            pub username: String,
            #[serde(default)]
            pub password: String,
            #[serde(default)]
            pub database: usize,
            #[serde(default)]
            pub max_pools: usize,
            #[serde(default)]
            pub minimum_idle_connections: usize,
            #[serde(default)]
            pub idle_timeout: usize,
            #[serde(default)]
            pub connect_timeout: usize,
        }
        let shadow = Shadow::deserialize(deserializer)?;
        let mut config = RedisConfig {
            host: shadow.host,
            unix_socket: shadow.unix_socket,
            port: shadow.port,
            username: shadow.username,
            password: shadow.password,
            database: shadow.database,
            max_pools: shadow.max_pools,
            minimum_idle_connections: shadow.minimum_idle_connections,
            idle_timeout: shadow.idle_timeout,
            connect_timeout: shadow.connect_timeout,
            __reconfigured: false,
        };
        config.__reconfigure();
        Ok(config)
    }
}

#[derive(Debug)]
pub struct RedisManager {
    config: Arc<RedisConfig>,
    pool: Mutex<Option<Pool<Client>>>,
}

impl IntoConnectionInfo for RedisConfig {
    fn into_connection_info(self) -> RedisResult<ConnectionInfo> {
        self.to_connection_info()
    }
}

impl RedisManager {
    pub fn new(config: RedisConfig) -> Self {
        Self {
            config: Arc::new(config),
            pool: Mutex::new(None),
        }
    }

    fn create_pool(&self) -> ResultError<Pool<Client>> {
        let conf = &self.config;
        let connection_info = conf.to_connection_info().map_err(Error::from_error)?;
        let thread_pool = Arc::new(ScheduledThreadPool::new(conf.max_pools));
        let client = Client::open(connection_info).map_err(Error::from_error)?;
        let pool = Pool::builder()
            .max_size(conf.max_pools as u32)
            .min_idle(Some(conf.minimum_idle_connections as u32))
            .idle_timeout(Some(Duration::from_secs(conf.idle_timeout as u64)))
            .connection_timeout(Duration::from_secs(conf.connect_timeout as u64))
            .build(client)
            .map_err(Error::from_error)?;
        Ok(pool)
    }

    pub fn get_pool(&self) -> ResultError<Pool<Client>> {
        let mut lock = self.pool.lock();
        if let Some(ref p) = *lock {
            return Ok(p.clone());
        }
        let new_pool = self.create_pool()?;
        *lock = Some(new_pool.clone());
        Ok(new_pool)
    }
    pub fn get_connection(&self) -> ResultError<PooledConnection<Client>> {
        self.get_pool()?.get().map_err(Error::from_error)
    }
    pub fn get_connection_with_timeout(
        &self,
        timeout: Duration,
    ) -> ResultError<PooledConnection<Client>> {
        self.get_pool()?
            .get_timeout(timeout)
            .map_err(Error::from_error)
    }
}
