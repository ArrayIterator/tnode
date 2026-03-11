use crate::cores::cache::cache::QueueGuard;
use crate::cores::cache::error::ResultCacheError;
use crate::cores::cache::traits::{CacheData, CacheItemPoolTrait, CacheItemTrait, CacheValue};
use crate::cores::helper::hash::HashTrait;
use chrono::{DateTime, TimeZone, Utc};
use dashmap::DashMap;
use std::collections::{HashMap, HashSet};
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;
use std::time::Duration;

#[derive(Debug)]
pub struct FileCacheItem {
    key: String,
    value: CacheValue,
    expiration: Option<DateTime<Utc>>,
    exists: bool,
    regenerating: bool,
    target: PathBuf,
    dirty: bool,
}

impl CacheItemTrait for FileCacheItem {
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

    fn set_expiration(&mut self, duration: Option<DateTime<Utc>>) {
        self.expiration = duration;
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

#[derive(Debug)]
pub struct FileSystem {
    pub path: PathBuf,
    pub default_expiration: Option<Duration>,
    pub deferred: DashMap<String, Arc<CacheData>>,
    in_queue_commit: Arc<AtomicBool>,
    in_queue_clear: Arc<AtomicBool>,
}

impl FileSystem {
    const EXTENSION: &'static str = ".cache";

    pub fn new(path: &Path, expiration: Option<Duration>) -> Self {
        Self {
            path: path.to_path_buf(),
            default_expiration: expiration,
            deferred: DashMap::with_capacity(1024),
            in_queue_commit: Arc::new(AtomicBool::new(false)),
            in_queue_clear: Arc::new(AtomicBool::new(false)),
        }
    }

    pub fn get_path(&self) -> &PathBuf {
        &self.path
    }

    fn get_or_create_dir(&self) -> ResultCacheError<&Path> {
        if self.path.exists() {
            return Ok(self.path.as_path());
        }
        std::fs::create_dir_all(&self.path)
            .map_err(|e| crate::cores::cache::error::CacheError::io_error(e.to_string()))?;
        Ok(self.path.as_path())
    }
    fn get_create_or_dir_target(&self, key: &str) -> ResultCacheError<PathBuf> {
        let path = self.get_or_create_dir()?;
        let path = path.join(key);
        if path.exists() {
            return Ok(path.to_path_buf());
        }
        std::fs::create_dir_all(&path)
            .map_err(|e| crate::cores::cache::error::CacheError::io_error(e.to_string()))?;
        Ok(path.to_path_buf())
    }
    fn gen_create_target(&self, key: &str) -> ResultCacheError<PathBuf> {
        let target = self.gen_target(key);
        std::fs::create_dir_all(&target)
            .map_err(|e| crate::cores::cache::error::CacheError::io_error(e.to_string()))?;
        Ok(target)
    }
    fn gen_target(&self, key: &str) -> PathBuf {
        let path = self.path.join(key);
        let sha = key.to_sha1();
        let first_char = sha.chars().next().unwrap();
        path.join(sha).join(Self::EXTENSION).to_path_buf()
    }
    fn create_default<T: AsRef<Path>>(
        &self,
        key: &str,
        path: T,
    ) -> ResultCacheError<Box<dyn CacheItemTrait>> {
        Ok(Box::new(FileCacheItem {
            key: key.to_string(),
            value: CacheValue::None,
            expiration: if let Some(d) = self.default_expiration {
                Some(Utc::now() + (chrono::Duration::seconds(d.as_secs() as i64)))
            } else {
                None
            },
            exists: false,
            regenerating: false,
            target: path.as_ref().to_path_buf(),
            dirty: true,
        }))
    }
}

impl CacheItemPoolTrait for FileSystem {
    fn get_item(&self, key: &str) -> ResultCacheError<Box<dyn CacheItemTrait>> {
        if let Some(arc_data) = self.deferred.get(key) {
            let data = arc_data.value();
            let ts = if data.expiration == 0 {
                None
            } else {
                Utc.timestamp_opt(data.expiration as i64, 0).single()
            };
            return Ok(Box::new(FileCacheItem {
                key: data.key.clone(),
                value: data.value.clone(),
                expiration: ts,
                exists: true,
                regenerating: false,
                target: self.gen_target(key).to_path_buf(),
                dirty: true,
            }));
        }
        let path = self.gen_target(key);
        if !path.exists() {
            return self.create_default(key, &path);
        }
        let data = std::fs::read_to_string(&path)
            .map_err(|e| crate::cores::cache::error::CacheError::io_error(e.to_string()))?;
        if let Ok(data) = serde_json::from_str::<CacheData>(&data).map_err(|e| {
            std::fs::remove_file(&path).ok();
            crate::cores::cache::error::CacheError::invalid_value(e.to_string())
        }) {
            let now = Utc::now().timestamp() as u64;
            if now > data.expiration {
                std::fs::remove_file(&path).ok();
                return self.create_default(key, &path);
            }
            let key_data = data.key.clone();
            if key != &key_data {
                std::fs::remove_file(&path).ok();
                return self.create_default(key, &path);
            }
            let ts = if data.expiration == 0 {
                None
            } else {
                Utc.timestamp_opt(data.expiration as i64, 0).single()
            };
            return Ok(Box::new(FileCacheItem {
                key: key.to_string(),
                value: data.value.into(),
                expiration: ts,
                exists: true,
                regenerating: false,
                target: path.to_path_buf(),
                dirty: false,
            }));
        }
        self.create_default(key, &path)
    }
    fn get_items(&self, keys: Vec<&str>) -> HashMap<String, Box<dyn CacheItemTrait>> {
        let unique_keys: HashSet<&str> = keys.into_iter().collect();
        let mut results: HashMap<String, Box<dyn CacheItemTrait>> =
            HashMap::with_capacity(unique_keys.len());
        for key in unique_keys {
            if let Some(arc_data) = self.deferred.get(key) {
                let data = arc_data.value();
                let ts = if data.expiration == 0 {
                    None
                } else {
                    Utc.timestamp_opt(data.expiration as i64, 0).single()
                };
                results.insert(
                    key.to_string(),
                    Box::new(FileCacheItem {
                        key: key.to_string(),
                        value: data.value.clone(),
                        expiration: ts,
                        exists: true,
                        regenerating: false,
                        target: self.gen_target(key).to_path_buf(),
                        dirty: true,
                    }),
                );
                continue;
            }
            if let Ok(item) = self.get_item(key) {
                results.insert(key.to_string(), item);
            }
        }
        results
    }
    fn save_item(&self, item: &dyn CacheItemTrait) -> ResultCacheError<bool> {
        let target = self.gen_create_target(item.get_key())?;
        let writer = std::fs::File::create(target)
            .map_err(|e| crate::cores::cache::error::CacheError::io_error(e.to_string()))?;
        let data = CacheData {
            key: item.get_key().to_string(),
            expiration: item
                .get_expiration()
                .map(|e| e.timestamp() as u64)
                .unwrap_or(0),
            value: item.get().unwrap_or(CacheValue::None),
        };

        serde_json::to_writer(writer, &data)
            .map_err(|e| crate::cores::cache::error::CacheError::conversion_error(e.to_string()))?;
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
        if self.in_queue_clear.swap(true, Ordering::SeqCst) {
            return Ok(false);
        }

        let _clear_guard = QueueGuard {
            flag: self.in_queue_clear.clone(),
        };

        self.deferred.clear();
        while self.in_queue_commit.load(Ordering::SeqCst) {
            std::thread::sleep(Duration::from_millis(10));
        }

        if self.path.exists() {
            let entries = std::fs::read_dir(&self.path)
                .map_err(|e| crate::cores::cache::error::CacheError::io_error(e.to_string()))?;
            for entry in entries {
                if let Ok(entry) = entry {
                    let path = entry.path();
                    if path.is_dir() {
                        let _ = std::fs::remove_dir_all(path);
                    } else {
                        let _ = std::fs::remove_file(path);
                    }
                }
            }
        }

        Ok(true)
    }

    fn delete_items(&self, key: Vec<&str>) -> ResultCacheError<bool> {
        let key_targets = key.iter().map(|k| self.gen_target(k)).collect::<Vec<_>>();
        for target in key_targets {
            if target.exists() {
                std::fs::remove_file(target).ok();
            }
        }
        Ok(true)
    }

    fn delete_item(&self, key: &str) -> ResultCacheError<bool> {
        let target = self.gen_target(key);
        if target.exists() {
            std::fs::remove_file(target).ok();
        }
        Ok(true)
    }

    fn commit(&self) -> ResultCacheError<bool> {
        if self.in_queue_commit.swap(true, Ordering::Acquire) {
            return Ok(false);
        }

        let _guard = QueueGuard {
            flag: self.in_queue_commit.clone(),
        };

        let mut items_to_save = Vec::new();

        while !self.deferred.is_empty() {
            if let Some(kv) = self.deferred.iter().next() {
                let key = kv.key().clone();
                if let Some((_, data)) = self.deferred.remove(&key) {
                    items_to_save.push(data);
                }
            }
        }

        if items_to_save.is_empty() {
            return Ok(true);
        }

        let mut failed_items = Vec::new();
        for item in items_to_save {
            let success = (|| -> ResultCacheError<()> {
                let target = self.gen_create_target(&item.key)?;
                let writer = std::fs::File::create(target)
                    .map_err(|e| crate::cores::cache::error::CacheError::io_error(e.to_string()))?;
                serde_json::to_writer(writer, &*item).map_err(|e| {
                    crate::cores::cache::error::CacheError::conversion_error(e.to_string())
                })?;
                Ok(())
            })();
            if success.is_err() {
                failed_items.push(item);
            }
        }

        let all_success = failed_items.is_empty();
        for item in failed_items {
            self.deferred.entry(item.key.clone()).or_insert(item);
        }

        Ok(all_success)
    }
}
