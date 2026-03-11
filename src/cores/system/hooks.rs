use std::{any::{Any, TypeId}, fmt::Debug, sync::Arc};
use dashmap::DashMap;
use crate::cores::{generator::uuid::Uuid, system::error::ResultError, system::error::Error};

pub const DEFAULT_PRIORITY: isize = 0;

pub type FnCallback<T> = fn(Arc<T>) -> ResultError<Arc<T>>;
pub struct HookItem {
    id: [u8;16],
    priority: isize,
    once: bool,
    type_id: TypeId,
    callback: Arc<dyn Any + Send + Sync>,
}

impl HookItem {
    pub fn new<T: Any + Send + Sync + 'static>(callback: FnCallback<T>, priority: isize, once: bool) -> Self {
        Self {
            id: Uuid::v7().to_bytes_le(),
            priority,
            once,
            type_id: TypeId::of::<T>(),
            callback: Arc::new(callback),
        }
    }
    pub fn id(&self) -> [u8;16] {
        self.id
    }
    pub fn priority(&self) -> isize {
        self.priority
    }
    pub fn once(&self) -> bool {
        self.once
    }
    pub fn type_id(&self) -> TypeId {
        self.type_id
    }
}

impl HookItem {
    fn get_callback<T: Any + Send + Sync + 'static>(&self) -> ResultError<FnCallback<T>> {
        self.callback
            .downcast_ref::<FnCallback<T>>()
            .map(|cb| *cb) // Copy fn pointer-e
            .ok_or_else(|| Error::invalid_data("Hook type mismatch, Su!"))
    }

    fn call<T: Any + Send + Sync>(&self, value: Arc<T>) -> ResultError<Arc<T>> {
        let e = (self.get_callback::<T>()?)(value.clone())?;
        Ok(e)
    }
}

impl PartialEq for HookItem {
    fn eq(&self, other: &Self) -> bool {
        self.type_id == other.type_id && self.id == other.id && self.once == other.once && Arc::ptr_eq(&self.callback, &other.callback)
    }
}

impl Debug for HookItem {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("HookItem")
            .field("type_id", &self.type_id)
            .field("value", &"Dynamic<Send + Sync + 'static>")
            .finish()
    }
}

#[derive(Debug, Default)]
pub struct Hooks {
    hooks: Arc<DashMap<String, Arc<DashMap<[u8;16], Arc<HookItem>>>>>,
    // used to prevent infinite loop when a hook is applied, it will store the current applied hooks in this map, and if a hook is applied again while it's already in the current_hook map, it will skip it to prevent infinite loop
    current_hook: Arc<DashMap<String, Arc<DashMap<[u8;16], bool>>>>,
}

impl Hooks {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn add_hook(&self, name: &str, item: HookItem) {
        self.hooks.entry(name.to_string()).or_insert_with(||Arc::new(DashMap::new())).insert(item.id, Arc::new(item));
    }

    pub fn add<T: Any + Send + Sync + 'static>(&self, name: &str, callback: FnCallback<T>, priority: isize, once: bool) {
        self.add_hook(name, HookItem::new(callback, priority, once));
    }

    pub fn add_once<T: Any + Send + Sync + 'static>(&self, name: &str, callback: FnCallback<T>, priority: isize) {
        self.add(name, callback, priority, true);
    }
    pub fn add_default<T: Any + Send + Sync + 'static>(&self, name: &str, callback: FnCallback<T>) {
        self.add(name, callback, DEFAULT_PRIORITY, false);
    }
    pub fn remove_by_id(&self, name: &str, id: [u8;16]) {
        if let Some(map) = self.hooks.get(name) {
            map.remove(&id);
        }
    }

    pub fn remove(&self, name: &str, priority: Option<isize>, type_id: Option<TypeId>) -> Vec<Arc<HookItem>>{
        if let Some(map) = self.hooks.get(name) {
            let items: Vec<Arc<HookItem>> = map.iter().filter(|item| {
                (priority.is_none() || item.value().priority == priority.unwrap()) &&
                (type_id.is_none() || item.value().type_id == type_id.unwrap())
            }).map(|item| item.value().clone()).collect();
            for item in &items {
                map.remove(&item.id);
            }
            items
        } else {
            vec![]
        }
    }

    pub fn in_hook(&self, name: &str, id: Option<Arc<HookItem>>) -> bool {
        if let Some(ids) = self.current_hook.get(name) {
            if let Some(id) = id {
                ids.contains_key(&id.id)
            } else {
                !ids.is_empty()
            }
        } else {
            false
        }
    }

    pub fn apply<T: Send + Sync + 'static>(&self, name: &str, hook: T) -> Arc<T> {
        let mut value: Arc<T> = Arc::new(hook);
        if let Some(map) = self.hooks.get(name) {
            let sorted = map.iter().filter(|item| item.value().type_id == TypeId::of::<T>()).map(|item| item.value().clone()).collect::<Vec<Arc<HookItem>>>();
            let mut sorted = sorted.into_iter().collect::<Vec<Arc<HookItem>>>();
            sorted.sort_by(|a, b| b.priority.cmp(&a.priority));
            let current = self.current_hook.entry(name.to_string()).or_insert_with(||Arc::new(DashMap::new()));
            for item in sorted {
                let id = item.id;
                if self.in_hook(name, Some(item.clone())) {
                    continue;
                }
                current.insert(id, true);
                if let Ok(hook) = item.call(value.clone()) {
                    value = hook;
                    if item.once {
                        map.remove(&item.id);
                    }
                }
                current.remove(&id);
            }
            if current.is_empty() {
                self.current_hook.remove(name);
            }
        }
        value
    }
}
