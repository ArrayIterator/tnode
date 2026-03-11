use crate::cores::cache::cache::{CacheItem, QueueGuard};
use crate::cores::cache::error::{CacheError, ResultCacheError};
use crate::cores::cache::traits::{CacheData, CacheItemPoolTrait, CacheItemTrait, CacheValue};
use crate::cores::libs::redis::RedisManager;
use chrono::{TimeZone, Utc};
use dashmap::DashMap;
use redis::Commands;
use std::collections::{HashMap, HashSet};
use std::fmt::Debug;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;
use std::time::Duration;

#[derive(Debug)]
pub struct Redis {
    redis_manager: Arc<RedisManager>,
    pub default_expiration: Option<Duration>,
    pub deferred: DashMap<String, Arc<CacheData>>,
    in_queue_commit: Arc<AtomicBool>,
    in_queue_clear: Arc<AtomicBool>,
}

impl Redis {
    pub fn new(redis_manager: Arc<RedisManager>, expiration: Option<Duration>) -> Self {
        Self {
            redis_manager,
            default_expiration: expiration,
            deferred: DashMap::with_capacity(1024),
            in_queue_commit: Arc::new(AtomicBool::new(false)),
            in_queue_clear: Arc::new(AtomicBool::new(false)),
        }
    }
    fn create_default_item(&self, key: &str) -> ResultCacheError<Box<dyn CacheItemTrait>> {
        Ok(Box::new(CacheItem {
            key: key.to_string(),
            value: CacheValue::None,
            expiration: self
                .default_expiration
                .map(|d| Utc::now() + chrono::Duration::from_std(d).unwrap()),
            exists: false,
            regenerating: false,
        }) as Box<dyn CacheItemTrait>)
    }
}

impl CacheItemPoolTrait for Redis {
    fn get_item(&self, key: &str) -> ResultCacheError<Box<dyn CacheItemTrait>> {
        let mut conn = self
            .redis_manager
            .get_connection()
            .map_err(|e| CacheError::io_error(e.to_string()))?;
        let data: Option<String> = conn
            .get(key)
            .map_err(|e| CacheError::io_error(e.to_string()))?;

        match data {
            Some(json_str) => {
                let data: CacheData = serde_json::from_str(&json_str)
                    .map_err(|e| CacheError::invalid_value(e.to_string()))?;

                // Cek Expiration (Redis biasanya handle TTL, tapi kita tetep validasi)
                let now = Utc::now().timestamp() as u64;
                if data.expiration > 0 && now > data.expiration {
                    let _: () = conn.del(key).ok().unwrap_or(());
                    return self.create_default_item(key);
                }

                Ok(Box::new(CacheItem {
                    key: data.key,
                    value: data.value,
                    expiration: if data.expiration == 0 {
                        None
                    } else {
                        Utc.timestamp_opt(data.expiration as i64, 0).single()
                    },
                    exists: true,
                    regenerating: false,
                }))
            }
            None => self.create_default_item(key),
        }
    }

    fn get_items(&self, keys: Vec<&str>) -> HashMap<String, Box<dyn CacheItemTrait>> {
        let unique_keys: HashSet<&str> = keys.into_iter().collect();
        let mut results = HashMap::with_capacity(unique_keys.len());
        let mut redis_keys = Vec::new();
        for key in unique_keys {
            if let Some(arc_data) = self.deferred.get(key) {
                let data = arc_data.value();
                results.insert(
                    key.to_string(),
                    Box::new(CacheItem {
                        key: key.to_string(),
                        value: data.value.clone(),
                        expiration: if data.expiration == 0 {
                            None
                        } else {
                            Utc.timestamp_opt(data.expiration as i64, 0).single()
                        },
                        exists: true,
                        regenerating: false,
                    }) as Box<dyn CacheItemTrait>,
                );
            } else {
                redis_keys.push(key);
            }
        }
        if !redis_keys.is_empty() {
            if let Ok(mut conn) = self.redis_manager.get_connection() {
                let values: Vec<Option<String>> = conn.mget(&redis_keys).unwrap_or_default();
                for (i, val) in values.into_iter().enumerate() {
                    let key = redis_keys[i];
                    if let Some(json_str) = val {
                        if let Ok(data) = serde_json::from_str::<CacheData>(&json_str) {
                            results.insert(
                                key.to_string(),
                                Box::new(CacheItem {
                                    key: data.key,
                                    value: data.value,
                                    expiration: if data.expiration == 0 {
                                        None
                                    } else {
                                        Utc.timestamp_opt(data.expiration as i64, 0).single()
                                    },
                                    exists: true,
                                    regenerating: false,
                                }) as Box<dyn CacheItemTrait>,
                            );
                        }
                    }
                }
            }
        }
        results
    }

    fn save_item(&self, item: &dyn CacheItemTrait) -> ResultCacheError<bool> {
        let mut conn = self
            .redis_manager
            .get_connection()
            .map_err(|e| CacheError::io_error(e.to_string()))?;

        let expiration_ts = item
            .get_expiration()
            .map(|e| e.timestamp() as u64)
            .unwrap_or(0);
        let data = CacheData {
            key: item.get_key().to_string(),
            expiration: expiration_ts,
            value: item.get().unwrap_or(CacheValue::None),
        };

        let json_str = serde_json::to_string(&data)
            .map_err(|e| CacheError::conversion_error(e.to_string()))?;

        if expiration_ts > 0 {
            let now = Utc::now().timestamp() as u64;
            let ttl = expiration_ts.saturating_sub(now);
            if ttl > 0 {
                let _: () = conn
                    .set_ex(item.get_key(), json_str, ttl as usize as u64)
                    .map_err(|e| CacheError::io_error(e.to_string()))?;
            } else {
                return Ok(false);
            }
        } else {
            let _: () = conn
                .set(item.get_key(), json_str)
                .map_err(|e| CacheError::io_error(e.to_string()))?;
        }

        Ok(true)
    }

    //noinspection DuplicatedCode
    fn save_deferred(&self, item: &dyn CacheItemTrait) -> bool {
        let data = CacheData {
            key: item.get_key().to_string(),
            expiration: item
                .get_expiration()
                .map(|e| e.timestamp() as u64)
                .unwrap_or(0),
            value: item.get().unwrap_or(CacheValue::None),
        };
        self.deferred.insert(data.key.clone(), Arc::new(data));
        true
    }

    fn clear(&self) -> ResultCacheError<bool> {
        let mut conn = self
            .redis_manager
            .get_connection()
            .map_err(|e| CacheError::io_error(e.to_string()))?;
        redis::cmd("FLUSHDB")
            .query::<()>(&mut *conn)
            .map_err(|e| CacheError::io_error(e.to_string()))?;
        Ok(true)
    }

    fn delete_items(&self, keys: Vec<&str>) -> ResultCacheError<bool> {
        let mut conn = self
            .redis_manager
            .get_connection()
            .map_err(|e| CacheError::io_error(e.to_string()))?;

        for key in &keys {
            self.deferred.remove(*key);
        }
        let _: () = conn
            .del(keys)
            .map_err(|e| CacheError::io_error(e.to_string()))?;
        Ok(true)
    }

    fn delete_item(&self, key: &str) -> ResultCacheError<bool> {
        let mut conn = self
            .redis_manager
            .get_connection()
            .map_err(|e| CacheError::io_error(e.to_string()))?;

        let _: () = conn
            .del(key)
            .map_err(|e| CacheError::io_error(e.to_string()))?;
        Ok(true)
    }
    fn commit(&self) -> ResultCacheError<bool> {
        if self.in_queue_commit.swap(true, Ordering::Acquire) {
            return Ok(false);
        }
        let _guard = QueueGuard {
            flag: self.in_queue_commit.clone(),
        };

        let mut conn = self
            .redis_manager
            .get_connection()
            .map_err(|e| CacheError::io_error(e.to_string()))?;

        let mut pipe = redis::pipe();
        let mut count = 0;
        while let Some(kv) = self.deferred.iter().next() {
            let key = kv.key().clone();
            if let Some((_, data)) = self.deferred.remove(&key) {
                let json_str = serde_json::to_string(&*data)
                    .map_err(|e| CacheError::conversion_error(e.to_string()))?;

                if data.expiration > 0 {
                    let now = Utc::now().timestamp() as u64;
                    let ttl = data.expiration.saturating_sub(now);
                    if ttl > 0 {
                        pipe.set_ex(&key, json_str, ttl as usize as u64);
                    }
                } else {
                    pipe.set(&key, json_str);
                }
                count += 1;
            }
        }

        if count > 0 {
            let _: () = redis::Pipeline::query(&pipe, &mut *conn)
                .map_err(|e| CacheError::io_error(e.to_string()))?;
        }

        Ok(true)
    }
}
