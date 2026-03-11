use crate::cores::cache::error::ResultCacheError;
use crate::cores::cache::traits::{CacheItemPoolTrait, CacheItemTrait, CacheValue};
use chrono::{DateTime, Utc};
use std::collections::HashMap;
use std::fmt::Debug;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;

pub trait CacheAdapter {}
#[derive(Debug)]
pub struct Cache {
    pool: Arc<dyn CacheItemPoolTrait>,
}

#[derive(Debug)]
pub struct CacheItem {
    pub key: String,
    pub value: CacheValue,
    pub expiration: Option<DateTime<Utc>>,
    pub exists: bool,
    pub regenerating: bool,
}

impl CacheItem {
    pub fn new(key: String, value: CacheValue, expiration: Option<DateTime<Utc>>) -> Self {
        Self {
            key,
            value,
            expiration,
            exists: false,
            regenerating: false,
        }
    }
}

impl CacheItemTrait for CacheItem {
    fn get_key(&self) -> &str {
        &self.key
    }

    fn get(&self) -> ResultCacheError<CacheValue> {
        Ok(self.value.clone())
    }

    fn set(&mut self, value: CacheValue) -> ResultCacheError<()> {
        self.value = value;
        Ok(())
    }

    fn get_expiration(&self) -> Option<DateTime<Utc>> {
        self.expiration
    }

    fn set_expiration(&mut self, expiration: Option<DateTime<Utc>>) {
        self.expiration = expiration
    }

    fn is_hit(&self) -> bool {
        self.exists() && !self.is_expired()
    }

    fn exists(&self) -> bool {
        self.exists
    }

    fn is_regenerating(&self) -> bool {
        self.regenerating
    }
}

impl Cache {
    
    pub fn get_pool(&self) -> Arc<dyn CacheItemPoolTrait> {
        self.pool.clone()
    }
}

impl CacheItemPoolTrait for Cache {
    fn get_item(&self, key: &str) -> ResultCacheError<Box<dyn CacheItemTrait>> {
        self.pool.get_item(key)
    }
    fn get_items(&self, keys: Vec<&str>) -> HashMap<String, Box<dyn CacheItemTrait>> {
        self.pool.get_items(keys)
    }
    fn save_item(&self, item: &dyn CacheItemTrait) -> ResultCacheError<bool> {
        self.pool.save_item(item)
    }
    fn save_deferred(&self, item: &dyn CacheItemTrait) -> bool {
        self.pool.save_deferred(item)
    }
    fn clear(&self) -> ResultCacheError<bool> {
        self.pool.clear()
    }
    fn delete_items(&self, key: Vec<&str>) -> ResultCacheError<bool> {
        self.pool.delete_items(key)
    }
    fn delete_item(&self, key: &str) -> ResultCacheError<bool> {
        self.pool.delete_item(key)
    }

    fn commit(&self) -> ResultCacheError<bool> {
        self.pool.commit()
    }
}


pub(crate) struct QueueGuard {
    pub flag: Arc<AtomicBool>,
}
impl Drop for QueueGuard {
    fn drop(&mut self) {
        self.flag.store(false, Ordering::Release);
    }
}
