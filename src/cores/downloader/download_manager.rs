use crate::cores::{downloader::{builder::Builder, item::Item, settings::{DEFAULT_CAPACITY, MAX_CAPACITY, MIN_CAPACITY}}, net::dns::{DnsChoice}, system::{
    error::ResultError,
    event_manager::EventManager,
}};
use dashmap::DashMap;
use parking_lot::RwLock;
use std::{sync::{Arc, OnceLock}, time::Duration};
use crate::cores::downloader::settings::Settings;

#[derive(Debug)]
pub struct DownloadManager {
    pub(crate) event_manager: Arc<EventManager>,
    pub(crate) items: RwLock<DashMap<String, Arc<Item>>>,
    self_arc: OnceLock<Arc<DownloadManager>>,
    settings: RwLock<Settings>,
}

impl Default for DownloadManager {
    fn default() -> Self {
        Self {
            items: RwLock::new(DashMap::with_capacity(DEFAULT_CAPACITY)),
            event_manager: Arc::new(EventManager::default()),
            settings: RwLock::new(Settings::default()),
            self_arc: OnceLock::new(),
        }
    }
}

impl DownloadManager {
    pub fn new<D: Into<Arc<DnsChoice>>>(
        capacity: usize,
        max_retries: usize,
        max_follow_redirect: usize,
        follow_redirect: bool,
        timeouts: Duration,
        dns: Option<D>,
        event_manager: Arc<EventManager>,
    ) -> Self where Self: Send + Sized {
        let capacity = capacity.clamp(MIN_CAPACITY, MAX_CAPACITY);
        Self {
            items: RwLock::new(DashMap::with_capacity(capacity)),
            event_manager: event_manager.clone(),
            settings: RwLock::new(Settings::new(
                max_retries,
                max_follow_redirect,
                follow_redirect,
                timeouts,
                dns,
            )),
            self_arc: OnceLock::new(),
        }
    }

    pub fn default_from_event_manager(event_manager: Arc<EventManager>) -> Self {
        Self {
            items: RwLock::new(DashMap::with_capacity(DEFAULT_CAPACITY)),
            event_manager: event_manager.clone(),
            settings: RwLock::new(Settings::default()),
            self_arc: OnceLock::new(),
        }
    }

    pub fn set_event_manager(&mut self, event_manager: Arc<EventManager>) -> &mut Self {
        self.event_manager = event_manager;
        self
    }
    pub fn get_event_manager(&self) -> Arc<EventManager> {
        self.event_manager.clone()
    }
    pub fn set_default_max_retries(&self, retries: usize) -> &Self {
        self.settings.write().set_default_max_retries(retries);
        self
    }
    pub fn set_default_max_redirects(&self, redirects: usize) -> &Self {
        self.settings.write().set_default_max_redirects(redirects);
        self
    }
    pub fn set_default_follow_redirect(&self, follow: bool) -> &Self {
        self.settings.write().set_default_follow_redirect(follow);
        self
    }
    pub fn set_default_timeouts(&self, timeouts: Duration) -> &Self {
        self.settings.write().set_default_timeouts(timeouts);
        self
    }
    pub fn set_default_dns<D: Into<Arc<DnsChoice>>>(&self, dns: Option<D>) -> &Self {
        self.settings.write().set_default_dns(dns);
        self
    }
    pub fn get_default_dns(&self) -> Option<Arc<DnsChoice>> {
        self.settings.read().default_dns.clone()
    }
    pub fn get_default_max_retries(&self) -> usize {
        self.settings.read().default_max_retries
    }
    pub fn get_default_max_redirects(&self) -> usize {
        self.settings.read().default_max_redirects
    }
    pub fn get_default_follow_redirect(&self) -> bool {
        self.settings.read().default_follow_redirect
    }
    pub fn get_default_timeouts(&self) -> Duration {
        self.settings.read().default_timeouts
    }
    pub fn set_capacity(&self, capacity: usize) -> usize {
        let mut write = self.items.write();
        let old_capacity = write.capacity();
        let min_capacity = write.len().max(MIN_CAPACITY);
        let capacity = capacity.clamp(min_capacity, MAX_CAPACITY);
        if old_capacity == capacity {
            return capacity;
        }
        // move old dashmap to new one with new capacity
        let mut new_items = DashMap::with_capacity(capacity);
        write.clone_into(&mut new_items);
        *write = new_items;
        capacity
    }
    pub fn is_full(&self) -> bool {
        let items = self.items.read();
        items.capacity() <= items.len()
    }
    pub fn get<T: AsRef<str>>(&self, id: T) -> Option<Arc<Item>> {
        self.items
            .read()
            .get(id.as_ref())
            .map(|entry| entry.value().clone())
    }

    pub fn has<T: AsRef<str>>(&self, id: T) -> bool {
        self.items.read().contains_key(id.as_ref())
    }

    pub(crate) fn attach(&self, item: Arc<Item>) -> Arc<Item> {
        let id = item.get_metadata().get_id().to_string();
        self.items.write().entry(id).or_insert(item.clone());
        item
    }

    pub(crate) fn detach<T: AsRef<str>>(&self, id: T) -> Option<Arc<Item>> {
        self.items.write().remove(id.as_ref()).map(|(_, item)| item)
    }

    pub fn builder<U: Into<String>>(self: &Arc<Self>, url: U) -> ResultError<Builder>{
        Builder::new(self.clone(), url)
    }
}
