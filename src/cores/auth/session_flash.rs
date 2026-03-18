use std::{any::{Any, type_name}, fmt::Display, sync::{Arc, atomic::{AtomicBool, AtomicU64, Ordering}}, time::{Duration, Instant, SystemTime}};
use dashmap::DashMap;
use crate::cores::{system::error::{Error, ResultError}};

// duration only allow min 5 seconds into 60 seconds, and the default duration is 10 seconds,
// this is to prevent abuse of flash messages and to ensure that they are used for short-term notifications only.
pub const MAX_DURATION: Duration = Duration::from_secs(60); // 1 year
pub const MIN_DURATION: Duration = Duration::from_secs(5); // 5 seconds
pub const DEFAULT_DURATION: Duration = Duration::from_secs(10); // 10 seconds
pub const MIN_THRESHOLD: u64 = 10; // minimum threshold is 10 items per kind
pub const MAX_THRESHOLD: u64 = 1000; // maximum threshold is 1000 items per kind
pub const DEFAULT_THRESHOLD: u64 = 100; // default threshold is 100 items per kind
pub const DELAY_DURATION: Duration = Duration::from_millis(100);
pub const WAIT_DURATION: Duration = Duration::from_millis(500);

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Kind {
    Success,
    Error,
    Warning,
    Info,
    Custom(String)
}

impl Display for Kind {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Kind::Success => write!(f, "Success"),
            Kind::Error => write!(f, "Error"),
            Kind::Warning => write!(f, "Warning"),
            Kind::Info => write!(f, "Info"),
            Kind::Custom(s) => write!(f, "{}", s),
        }
    }
}
impl From<String> for Kind {
    fn from(value: String) -> Self {
        let lower = value.to_lowercase();
        match lower.as_str() {
            "success" => Kind::Success,
            "error" => Kind::Error,
            "warning" => Kind::Warning,
            "info" => Kind::Info,
            _ => Kind::Custom(value)
        }
    }
}

impl From<&str> for Kind {
    fn from(value: &str) -> Self {
        let lower = value.to_lowercase();
        match lower.as_str() {
            "success" => Kind::Success,
            "error" => Kind::Error,
            "warning" => Kind::Warning,
            "info" => Kind::Info,
            _ => Kind::Custom(value.to_string())
        }
    }
}

#[derive(Debug, Clone)]
pub struct FlashMessage(pub String);

impl From<String> for FlashMessage {
    fn from(value: String) -> Self {
        FlashMessage(value)
    }
}

impl From<&str> for FlashMessage {
    fn from(value: &str) -> Self {
        FlashMessage(value.to_string())
    }
}

#[derive(Debug, Clone, Default)]
pub struct FlashMetadata {
    metadata: Arc<DashMap<String, Arc<dyn Any + Send + Sync + 'static>>>
}

#[derive(Debug, Clone)]
pub struct FlashItem {
    pub(crate) id: String,
    pub(crate) time: Instant,
    pub kind: Kind,
    pub message: FlashMessage,
    pub metadata: FlashMetadata
}

#[derive(Debug, Clone, Default)]
pub struct FlashItems {
    duration: Arc<AtomicU64>,
    threshold: Arc<AtomicU64>,
    items: Arc<DashMap<String, Arc<FlashItem>>>,
    atomic_cleaning: Arc<AtomicBool>
}

#[derive(Debug, Clone)]
pub struct Flash {
    id: String,
    duration: Arc<AtomicU64>,
    threshold: Arc<AtomicU64>,
    items: Arc<DashMap<String, Arc<FlashItems>>>,
    atomic_cleaning: Arc<AtomicBool>,
    atomic_last_recycle: Arc<AtomicU64>, // nano durations
    atomic_join_recycle: Arc<AtomicBool>
}

impl FlashMetadata {
    pub fn set<I: Into<String>, V: Any + Send + Sync + 'static>(&self, key: I, value: V) -> Arc<V> {
        let value = Arc::new(value);
        self.metadata.insert(key.into(), value.clone());
        value
    }
    pub fn remove<I: Into<String>>(&self, key: I) -> Option<Arc<dyn Any + Send + Sync + 'static>> {
        self.metadata.remove(&key.into()).map(|(_, e)|e)
    }
    pub fn hash<I: Into<String>>(&self, key: I) -> bool {
        self.metadata.contains_key(&key.into())
    }
    pub fn clear(&self) -> usize {
        let length = self.metadata.len();
        self.metadata.clear();
        length
    }
    pub fn get<I: Into<String>>(&self, key: I) -> Option<Arc<dyn Any + Send + Sync + 'static>> {
        self.metadata.get(&key.into()).map(|e|e.clone())
    }
    pub fn try_cast<I: Into<String>, C:  Send + Sync + 'static>(&self, key: I) -> ResultError<Arc<C>> {
        let key = key.into();
        if let Some(m) = self.get(key.clone()) {
            let c = m.downcast::<C>().map_err(|e|{
                Error::other(format!("Failed to downcast metadata into : {}", type_name::<C>()))
            })?;
            return Ok(c.clone())
        }
        Err(Error::not_found(format!("Metadata not found for: {}", key)))
    }
}

impl FlashItem {
    pub fn new<Id: Into<String>, K: Into<Kind>, F: Into<FlashMessage>>(id: Id, kind:K, message: F, metadata: Option<FlashMetadata>) -> Self {
        Self {
            id: id.into(),
            kind: kind.into(),
            time: Instant::now(),
            message: message.into(),
            metadata: if let Some(metadata) = metadata {
                metadata
            } else {
                FlashMetadata::default()
            },
        }
    }
    pub fn is_expired(&self, duration: Duration) -> bool {
        self.time.elapsed() >= duration
    }
    pub fn id(&self) -> &str {
        &self.id
    }
}

impl FlashItems {
    pub fn new(duration: Duration, threshold: u64) -> Self {
        let duration = duration.clamp(MIN_DURATION, MAX_DURATION);
        let threshold = threshold.clamp(MIN_THRESHOLD, MAX_THRESHOLD);
        FlashItems {
            duration: Arc::new(AtomicU64::new(duration.as_nanos() as u64)),
            threshold: Arc::new(AtomicU64::new(threshold)),
            items: Arc::new(DashMap::new()),
            atomic_cleaning: Arc::new(AtomicBool::new(false)),
        }
    }
    pub fn threshold(&self) -> u64 {
        self.threshold.load(Ordering::SeqCst)
    }
    pub fn duration(&self) -> Duration {
        Duration::from_nanos(self.duration.load(Ordering::SeqCst))
    }
    pub fn add(&self, item: FlashItem) -> Arc<FlashItem> {
        let item = Arc::new(item);
        self.add_arc(item)
    }
    pub fn add_arc(&self, item: Arc<FlashItem>) -> Arc<FlashItem> {
        let item = item.clone();
        self.items.insert(item.id.clone(), item.clone());
        self.threshold_recycle();
        item
    }
    pub fn remove<I: Into<String>>(&self, key: I) -> Option<Arc<FlashItem>> {
        self.items.remove(&key.into()).map(|(_, e)| e)
    }
    pub fn get<I:Into<String>>(&self, key: I) -> Option<Arc<FlashItem>> {
        self.items.get(&key.into()).map(|e|e.clone())
    }

    pub fn items(&self) -> Vec<Arc<FlashItem>> {
        let duration = self.duration();
        self.items.retain(|_, item| !item.is_expired(duration));
        self.items.iter().map(|e| e.value().clone()).collect()
    }

    pub fn clear(&self) -> usize { // returning previously retained items count
        self.atomic_cleaning.store(true, Ordering::SeqCst);
        let length = self.items.len();
        self.items.clear();
        self.atomic_cleaning.store(false, Ordering::SeqCst);
        length
    }
    pub fn threshold_recycle(&self) -> usize {
        let len = self.items.len();
        if len >= self.threshold() as usize {
            self.recycle()
        } else {
            len
        }
    }
    pub fn recycle(&self) -> usize { // returning retained items count
        if self.atomic_cleaning.swap(true, Ordering::SeqCst) {
            return self.items.len();
        }
        self.items.retain(|_, e| !e.is_expired(self.duration()));
        self.atomic_cleaning.store(false, Ordering::SeqCst);
        self.items.len()
    }
}

impl Flash {
    pub fn new(id: &str, duration: Duration, threshold: u64) -> Self {
        let duration = duration.clamp(MIN_DURATION, MAX_DURATION).as_nanos() as u64;
        let threshold = threshold.clamp(MIN_THRESHOLD, MAX_THRESHOLD);
        Flash {
            id: id.to_string(),
            duration: Arc::new(AtomicU64::new(duration)),
            threshold: Arc::new(AtomicU64::new(threshold)),
            items: Arc::new(DashMap::new()),
            atomic_cleaning: Arc::new(AtomicBool::new(false)),
            atomic_last_recycle: Arc::new(AtomicU64::new(0)),
            atomic_join_recycle: Arc::new(AtomicBool::new(false))
        }
    }
    pub fn id(&self) -> &str {
        &self.id
    }
    pub fn recycle(&self) -> usize {
        if self.atomic_cleaning.swap(true, Ordering::SeqCst) {
            return self.items.len();
        }
        let now_nanos = SystemTime::now()
            .duration_since(SystemTime::UNIX_EPOCH)
            .unwrap()
            .as_nanos() as u64;
        self.atomic_last_recycle.store(now_nanos, Ordering::SeqCst);
        self.items.retain(|_, e| {
            e.recycle() > 0
        });
        self.atomic_cleaning.store(false, Ordering::SeqCst);
        self.items.len()
    }
    pub fn threshold_recycle(&self) -> usize {
        let len = self.items.len();
        let last = self.atomic_last_recycle.load(Ordering::SeqCst);
        let now = SystemTime::now()
            .duration_since(SystemTime::UNIX_EPOCH)
            .unwrap()
            .as_nanos() as u64;
        let now = SystemTime::now()
            .duration_since(SystemTime::UNIX_EPOCH)
            .unwrap()
            .as_nanos() as u64;
        if len >= self.threshold() as usize && now.saturating_sub(last) >= self.duration().as_nanos() as u64 {
            let len = self.recycle();
            self.try_spawn_recycle();
            len
        } else {
            len
        }
    }
    async fn try_spawn_recycle(&self) {
        if self.atomic_join_recycle.swap(true, Ordering::SeqCst) {
            return;
        }
        let items = self.items.clone();
        let atomic_cleaning = self.atomic_cleaning.clone();
        let atomic_join_recycle = self.atomic_join_recycle.clone();
        let this = self.clone();
        tokio::spawn(async move {
            loop {
                let len = items.len();
                if len == 0 || !atomic_join_recycle.load(Ordering::SeqCst) { // if force to false
                    break;
                }
                if atomic_cleaning.load(Ordering::SeqCst) {
                    tokio::time::sleep(DELAY_DURATION).await;
                    continue;
                }
                if len >= this.threshold() as usize {
                    if this.recycle() == 0 {
                        break;
                    }
                }
                tokio::time::sleep(WAIT_DURATION).await;
            }
            atomic_join_recycle.store(false, Ordering::SeqCst);
        });
    }

    pub fn duration(&self) -> Duration {
        Duration::from_nanos(self.duration.load(Ordering::SeqCst))
    }
    pub fn threshold(&self) -> u64 {
        self.threshold.load(Ordering::SeqCst)
    }
    pub fn set_duration(&self, duration: Duration) -> &Self {
        let duration =  duration.clamp(MIN_DURATION, MAX_DURATION).as_nanos() as u64;
        self.duration.store(duration, Ordering::SeqCst);
        // change all flash items duration
        for entry in self.items.iter() {
            entry.duration.store(duration, Ordering::SeqCst);
        }
        self
    }
    pub fn set_threshold(&self, threshold: u64) -> &Self {
        let threshold = threshold.clamp(MIN_THRESHOLD, MAX_THRESHOLD);
        self.threshold.store(threshold, Ordering::SeqCst);
        // change all flash items threshold
        for entry in self.items.iter() {
            entry.threshold.store(threshold, Ordering::SeqCst);
        }
        self
    }
    pub fn add_arc(&self, item: Arc<FlashItem>) -> Arc<FlashItem> {
        if item.is_expired(self.duration()) {
            return item;
        }
        let item = item.clone();
        let kind = item.kind.to_string();
        self.items.entry(kind).or_insert_with(|| {
            let items = Arc::new(FlashItems::new(self.duration(), self.threshold.load(Ordering::SeqCst)));
            items.add_arc(item.clone());
            items
        });
        self.threshold_recycle();
        item
    }
    pub fn add(&self, item: FlashItem) -> Arc<FlashItem> {
        let item = Arc::new(item);
        self.add_arc(item.clone())
    }

    pub fn remove<K: Into<Kind>>(&self, kind: K) -> Option<Arc<FlashItems>> {
        let key = kind.into().to_string();
        self.items.remove(&key).map(|(_, e)| e)
    }

    pub fn get<K: Into<Kind>>(&self, kind: K) -> Option<Arc<FlashItems>> {
        let key = kind.into().to_string();
        self.items.get(&key).map(|e|e.clone())
    }

    pub fn clear(&self) -> usize {
        let mut count = 0;
        for entry in self.items.iter() {
            let items = entry.value();
            count += items.clear();
        }
        count
    }
    pub fn flash<Id: Into<String>, K: Into<Kind>, M: Into<FlashMessage>>(&self, id: Id, kind: K, message: M, metadata: Option<FlashMetadata>) -> Arc<FlashItem> {
        let item = FlashItem::new(id, kind, message, metadata);
        self.add(item)
    }
    pub fn flash_message<Id: Into<String>, K: Into<Kind>, M: Into<FlashMessage>>(&self, id: Id, kind: K, message: M) -> Arc<FlashItem> {
        self.flash(id, kind, message, None)
    }
    pub fn flash_success<Id: Into<String>,M: Into<FlashMessage>>(&self, id: Id, message: M) -> Arc<FlashItem> {
        self.flash(id, Kind::Success, message, None)
    }
    pub fn flash_error<Id: Into<String>,M: Into<FlashMessage>>(&self, id: Id, message: M) -> Arc<FlashItem> {
        self.flash(id, Kind::Error, message, None)
    }
    pub fn flash_warning<Id: Into<String>,M: Into<FlashMessage>>(&self, id: Id, message: M) -> Arc<FlashItem> {
        self.flash(id, Kind::Warning, message, None)
    }
    pub fn flash_info<Id: Into<String>,M: Into<FlashMessage>>(&self, id: Id, message: M) -> Arc<FlashItem> {
        self.flash(id, Kind::Info, message, None)
    }
    pub fn extend(&self, items: Vec<FlashItem>) -> Vec<Arc<FlashItem>> {
        items.into_iter().map(|item| self.add(item)).collect()
    }
    pub fn items(&self) -> Vec<Arc<FlashItems>> {
        self.items.iter().map(|e|e.clone()).collect()
    }

    pub fn merge_flash(&self, other: &Self) -> usize {
        let mut count = 0;
        for entry in other.items.iter() {
            let items = entry.value();
            let id = entry.key();
            let duration = self.duration();
            let i = self.items.entry(id.to_string()).or_insert_with(||{
                Arc::new(FlashItems::new(duration, self.threshold.load(Ordering::SeqCst)))
            });
            for item in items.items() {
                if item.is_expired(duration) {
                    continue;
                }
                let item = item.clone();
                let kind = item.kind.to_string();
                i.items.insert(item.id.clone(), item.clone());
                count += 1;
            }
            if i.items.len() == 0 {
                self.items.remove(id);
                continue;
            }
        }
        count
    }
}

#[derive(Debug, Clone)]
pub struct FlashManager {
    pub(crate) session_name: String,
    pub(crate) items: Arc<DashMap<String, Arc<Flash>>>,
    pub(crate) duration: Arc<AtomicU64>,
    pub(crate) threshold: Arc<AtomicU64>,
    atomic_cleaning: Arc<AtomicBool>,
    atomic_last_recycle: Arc<AtomicU64>, // nano durations
    atomic_join_recycle: Arc<AtomicBool>
}

impl FlashManager {
    pub fn new(session_name: &str, duration: Duration, threshold: u64) -> Self {
        let duration = duration.clamp(MIN_DURATION, MAX_DURATION).as_nanos() as u64;
        let threshold = threshold.clamp(MIN_THRESHOLD, MAX_THRESHOLD);
        FlashManager {
            session_name: session_name.to_string(),
            items: Arc::new(DashMap::new()),
            duration: Arc::new(AtomicU64::new(duration)),
            threshold: Arc::new(AtomicU64::new(threshold)),
            atomic_cleaning: Arc::new(AtomicBool::new(false)),
            atomic_last_recycle: Arc::new(AtomicU64::new(0)),
            atomic_join_recycle: Arc::new(AtomicBool::new(false))
        }
    }
    pub fn threshold(&self) -> u64 {
        self.threshold.load(Ordering::SeqCst)
    }
    pub fn duration(&self) -> Duration {
        Duration::from_nanos(self.duration.load(Ordering::SeqCst))
    }
    pub fn recycle(&self) -> usize {
        if self.atomic_cleaning.swap(true, Ordering::SeqCst) {
            return self.items.len();
        }
        let now_nanos = SystemTime::now()
            .duration_since(SystemTime::UNIX_EPOCH)
            .unwrap()
            .as_nanos() as u64;
        self.atomic_last_recycle.store(now_nanos, Ordering::SeqCst);
        self.items.retain(|_, e| {
            e.recycle() > 0
        });
        self.atomic_cleaning.store(false, Ordering::SeqCst);
        self.items.len()
    }
    pub fn threshold_recycle(&self) -> usize {
        let len = self.items.len();
        let last = self.atomic_last_recycle.load(Ordering::SeqCst);
        let now = SystemTime::now()
            .duration_since(SystemTime::UNIX_EPOCH)
            .unwrap()
            .as_nanos() as u64;
        let now = SystemTime::now()
            .duration_since(SystemTime::UNIX_EPOCH)
            .unwrap()
            .as_nanos() as u64;
        if len >= self.threshold() as usize && now.saturating_sub(last) >= self.duration().as_nanos() as u64 {
            let len = self.recycle();
            self.try_spawn_recycle();
            len
        } else {
            len
        }
    }
    async fn try_spawn_recycle(&self) {
        if self.atomic_join_recycle.swap(true, Ordering::SeqCst) {
            return;
        }
        let items = self.items.clone();
        let atomic_cleaning = self.atomic_cleaning.clone();
        let atomic_join_recycle = self.atomic_join_recycle.clone();
        let this = self.clone();
        tokio::spawn(async move {
            loop {
                let len = items.len();
                if len == 0 || !atomic_join_recycle.load(Ordering::SeqCst) { // if force to false
                    break;
                }
                if atomic_cleaning.load(Ordering::SeqCst) {
                    tokio::time::sleep(DELAY_DURATION).await;
                    continue;
                }
                if len >= this.threshold() as usize {
                    if this.recycle() == 0 {
                        break;
                    }
                }
                tokio::time::sleep(WAIT_DURATION).await;
            }
            atomic_join_recycle.store(false, Ordering::SeqCst);
        });
    }

    pub fn set_duration(&self, duration: Duration) -> &Self {
        let duration = duration.clamp(MIN_DURATION, MAX_DURATION).as_nanos() as u64;
        self.duration.store(duration, Ordering::SeqCst);
        // change all flash items duration
        for entry in self.items.iter() {
            entry.duration.store(duration, Ordering::SeqCst);
        }
        self
    }
    pub fn set_threshold(&self, threshold: u64) -> &Self {
        let threshold = threshold.clamp(MIN_THRESHOLD, MAX_THRESHOLD);
        self.threshold.store(threshold, Ordering::SeqCst);
        // change all flash items threshold
        for entry in self.items.iter() {
            entry.threshold.store(threshold, Ordering::SeqCst);
        }
        self
    }
    pub fn use_default(session_name: &str) -> Self {
        FlashManager::new(session_name, DEFAULT_DURATION, DEFAULT_THRESHOLD)
    }
    pub fn get<I: Into<String>>(&self, session_id: I) -> Arc<Flash> {
        let session_id = session_id.into();
        self.items.entry(session_id.clone()).or_insert_with(|| Arc::new(Flash::new(&session_id, self.duration(), self.threshold()))).clone()
    }
    pub fn remove<I: Into<String>>(&self, session_id: I) -> Option<Arc<Flash>> {
        self.items.remove(&session_id.into()).map(|(_, e)|e.clone())
    }
    pub fn remove_for<I: Into<String>>(&self, flash: &Flash) -> Option<Arc<Flash>> {
        self.remove(flash.id())
    }
    pub fn replace(&self, flash: Flash) -> Arc<Flash> {
        self.replace_arc(Arc::new(flash))
    }
    pub fn replace_arc(&self, flash: Arc<Flash>) -> Arc<Flash> {
        let session_id = flash.id().to_string();
        self.items.insert(session_id, flash.clone());
        self.threshold_recycle();
        flash
    }
    pub fn append(&self, flash: Flash) -> Arc<Flash> {
        self.append_arc(Arc::new(flash))
    }
    pub fn append_arc(&self, flash: Arc<Flash>) -> Arc<Flash> {
        let session_id = flash.id().to_string();
        let items = self.items.entry(session_id).or_insert_with(|| flash.clone());
        items.merge_flash(&flash);
        self.threshold_recycle();
        items.clone()
    }
    pub fn move_flash<Source: Into<String>, Target: Into<String>>(
        &self,
        source: Source,
        target: Target,
    ) -> Option<Arc<Flash>> {
        let source_id = source.into();
        let target_id = target.into();
        if let Some((_, flash)) = self.items.remove(&source_id) {
            let flash_items = if let Some(entry) = self.items.get(&target_id) {
                let extended_flash = entry.value(); //  &Arc<Flash>
                extended_flash.merge_flash(&flash);
                Some(extended_flash.clone())
            } else {
                let new_flash = Arc::new(Flash::new(&target_id, self.duration(), self.threshold()));
                new_flash.merge_flash(&flash);
                self.items.insert(target_id, new_flash.clone());
                Some(new_flash)
            };
            flash_items
        } else {
            None
        }
    }

    pub fn flash<S: Into<String>, Id: Into<String>, K: Into<Kind>, M: Into<FlashMessage>>(&self, session_id: S, id: Id, kind: K, message: M, metadata: Option<FlashMetadata>) -> Arc<FlashItem> {
        self.get(session_id).flash(id, kind, message, metadata)
    }
    pub fn flash_message<S: Into<String>, Id: Into<String>, K: Into<Kind>, M: Into<FlashMessage>>(&self, session_id: S, id: Id, kind: K, message: M) -> Arc<FlashItem> {
        self.get(session_id).flash_message(id, kind, message)
    }
    pub fn flash_success<S: Into<String>, Id: Into<String>,M: Into<FlashMessage>>(&self, session_id: S, id: Id, message: M) -> Arc<FlashItem> {
        self.get(session_id).flash_success(id, message)
    }
    pub fn flash_error<S: Into<String>, Id: Into<String>,M: Into<FlashMessage>>(&self, session_id: S, id: Id, message: M) -> Arc<FlashItem> {
        self.get(session_id).flash_error(id, message)
    }
    pub fn flash_warning<S: Into<String>, Id: Into<String>,M: Into<FlashMessage>>(&self, session_id: S, id: Id, message: M) -> Arc<FlashItem> {
        self.get(session_id).flash_warning(id, message)
    }
    pub fn flash_info<S: Into<String>, Id: Into<String>,M: Into<FlashMessage>>(&self, session_id: S, id: Id, message: M) -> Arc<FlashItem> {
        self.get(session_id).flash_info(id, message)
    }
}
