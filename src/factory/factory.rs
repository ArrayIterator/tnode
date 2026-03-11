use crate::cores::assets::assets::Assets;
use crate::cores::auth::totp::TimeBasedOneTimePassword;
use crate::cores::l10n::translator::Translator;
use crate::cores::libs::multipart::Multipart;
use crate::cores::runner::cli::Cli;
use crate::cores::system::commander::ControlCommander;
use crate::cores::downloader::download_manager::DownloadManager;
use crate::cores::system::error::{Error, ResultError};
use crate::cores::system::event_manager::EventManager;
use crate::cores::system::hooks::Hooks;
use crate::cores::system::middleware_manager::MiddlewareManager;
use crate::cores::system::routes::Routes;
use crate::cores::theme::themes::Themes;
use crate::factory::app::App;
use crate::factory::server::Server;
use dashmap::DashMap;
use std::any::{Any, TypeId, type_name};
use std::collections::HashMap;
use std::panic;
use std::sync::{Arc, LazyLock, OnceLock};

static INSTANCE_FACTORY: OnceLock<Arc<Factory>> = OnceLock::new();
static PROTECTED_TYPES: LazyLock<HashMap<TypeId, fn() -> Arc<dyn Any + Send + Sync>>> =
    LazyLock::new(|| {
        let tid: Vec<(TypeId, fn() -> Arc<dyn Any + Send + Sync>)> = vec![
            (TypeId::of::<Factory>(), || Factory::instance().clone()),
            (TypeId::of::<App>(), || App::instance().clone()),
            (TypeId::of::<Assets>(), || Arc::new(Assets::default())),
            (TypeId::of::<Cli>(), || Arc::new(Cli::default())),
            (TypeId::of::<ControlCommander>(), || {
                Arc::new(ControlCommander::default())
            }),
            (TypeId::of::<DownloadManager>(), || {
                Arc::new(DownloadManager::default_from_event_manager(
                    Factory::pick_unsafe::<EventManager>(),
                ))
            }),
            (TypeId::of::<EventManager>(), || {
                Arc::new(EventManager::default())
            }),
            (TypeId::of::<Hooks>(), || Arc::new(Hooks::default())),
            (TypeId::of::<MiddlewareManager>(), || {
                Arc::new(MiddlewareManager::default())
            }),
            (TypeId::of::<Multipart>(), || Arc::new(Multipart::default())),
            (TypeId::of::<Routes>(), || Arc::new(Routes::default())),
            (TypeId::of::<Server>(), || Arc::new(Server::default())),
            (TypeId::of::<Themes>(), || Arc::new(Themes::new())),
            (TypeId::of::<TimeBasedOneTimePassword>(), || {
                Arc::new(TimeBasedOneTimePassword::default())
            }),
            (TypeId::of::<Translator>(), || {
                Arc::new(Translator::default())
            }),
        ];
        HashMap::from_iter(tid.into_iter())
    });

#[derive(Debug)]
pub struct Factory {
    registry: Arc<DashMap<TypeId, Arc<dyn Any + Send + Sync>>>,
}

impl Factory {}

impl Factory {
    pub fn instance() -> Arc<Factory> {
        INSTANCE_FACTORY
            .get_or_init(|| {
                Arc::new(Self {
                    registry: Arc::new(DashMap::new()),
                })
            })
            .clone()
    }
    pub fn protected<T: 'static + Send + Sync>() -> bool {
        PROTECTED_TYPES.contains_key(&TypeId::of::<T>())
    }
    pub fn pick<T: 'static + Send + Sync>() -> ResultError<Arc<T>> {
        Self::instance().get::<T>()
    }
    pub fn pick_unsafe<T: 'static + Send + Sync>() -> Arc<T> {
        Self::instance().get_unsafe::<T>()
    }
    pub fn pick_mut<T: 'static + Send + Sync>() -> ResultError<&'static mut T> {
        let arc = Self::pick::<T>()?;
        unsafe { Ok(&mut *(Arc::as_ptr(&arc) as *mut T)) }
    }
    pub fn pick_mut_fn<T: 'static + Send + Sync, R>(f: fn(&mut T) -> R) -> ResultError<R> {
        Self::instance().get_mut_fn(f)
    }
    pub fn pick_or_set<T: 'static + Send + Sync>(func: impl FnOnce() -> T) -> Arc<T> {
        Self::instance().get_or_set::<T>(func)
    }
    pub fn pick_or_register<T: 'static + Send + Sync + Default>() -> Arc<T> {
        Self::instance().get_or_insert::<T>()
    }
    pub fn register_default<T: 'static + Send + Sync + Default>() -> Arc<T> {
        Self::instance().insert_default::<T>()
    }
    pub fn register<T: 'static + Send + Sync>(instance: T) -> Arc<T> {
        Self::instance().insert(instance)
    }
    pub fn registered<T: 'static + Send + Sync>() -> bool {
        Self::instance().exists::<T>()
    }
}

impl Factory {
    pub fn remove<T: 'static + Send + Sync>(&self) -> ResultError<Option<Arc<T>>> {
        if Self::protected::<T>() {
            return Err(Error::permission_denied(format!(
                "Cannot remove protected instance: {:?}",
                type_name::<T>()
            )));
        }
        let r = self.registry.remove(&TypeId::of::<T>());
        if let Some((_, instance)) = r {
            let instance = instance.downcast::<T>().map_err(|_| {
                Error::other(format!("Failed to remove instance {}", type_name::<T>()))
            })?;
            return Ok(Some(instance));
        }
        Ok(None)
    }

    pub fn insert_default<T: 'static + Send + Sync + Default>(&self) -> Arc<T> {
        self.insert_arc(Arc::new(T::default()))
    }

    pub fn insert<T: 'static + Send + Sync>(&self, instance: T) -> Arc<T> {
        self.insert_arc(Arc::new(instance))
    }

    pub fn insert_arc<T: 'static + Send + Sync>(&self, instance: Arc<T>) -> Arc<T> {
        let type_id = TypeId::of::<T>();
        if PROTECTED_TYPES.contains_key(&type_id) && self.exists::<T>() {
            return self.get_unsafe::<T>();
        }
        self.registry.insert(type_id, instance.clone());
        instance.clone()
    }

    pub fn exists<T: 'static + Send + Sync>(&self) -> bool {
        self.registry.contains_key(&TypeId::of::<T>())
    }

    pub fn contains<T: 'static + Send + Sync>(&self, instance: T) -> bool {
        self.contains_arc::<T>(Arc::new(instance))
    }

    pub fn contains_arc<T: 'static + Send + Sync>(&self, instance: Arc<T>) -> bool {
        self.get::<T>()
            .map(|this| Arc::ptr_eq(&this, &instance))
            .unwrap_or(false)
    }

    pub fn get_or_set<T: 'static + Send + Sync>(&self, func: impl FnOnce() -> T) -> Arc<T> {
        match self
            .registry
            .get(&TypeId::of::<T>())
            .map(|e| e.clone().downcast::<T>())
        {
            Some(e) => {
                if let Ok(e) = e {
                    e
                } else {
                    self.insert_arc(Arc::new(func()))
                }
            }
            None => self.insert_arc(Arc::new(func())),
        }
    }

    pub fn get_or_insert<T: 'static + Send + Sync + Default>(&self) -> Arc<T> {
        if let Some(e) = self.registry.get(&TypeId::of::<T>()) {
            if let Ok(e) = e.clone().downcast::<T>() {
                return e;
            }
        }
        let new_instance = Arc::new(T::default());
        self.registry
            .insert(TypeId::of::<T>(), new_instance.clone());
        new_instance
    }

    pub fn get_mut_fn<T: 'static + Send + Sync, R>(&self, f: fn(&mut T) -> R) -> ResultError<R> {
        let arc = self.get_mut::<T>()?;
        Ok(f(arc))
    }

    pub fn get_unsafe<T: 'static + Send + Sync>(&self) -> Arc<T> {
        let type_id = TypeId::of::<T>();
        self.registry
            .entry(type_id)
            .or_insert_with(|| {
                PROTECTED_TYPES
                    .get(&type_id)
                    .map(|e| e())
                    .unwrap_or_else(|| {
                        panic!("Not found instance for: {:?}", type_name::<T>());
                    })
            })
            .clone()
            .downcast::<T>()
            .expect(&format!(
                "Final downcast failed for type {:?}",
                type_name::<T>()
            ))
    }

    pub fn get_mut<T: 'static + Send + Sync>(&self) -> ResultError<&mut T> {
        let arc = self.get::<T>()?.clone();
        unsafe { Ok(&mut *(Arc::as_ptr(&arc) as *mut T)) }
    }

    pub fn get<T: 'static + Send + Sync>(&self) -> ResultError<Arc<T>> {
        let result = panic::catch_unwind(panic::AssertUnwindSafe(|| self.get_unsafe::<T>()));
        match result {
            Ok(instance) => Ok(instance),
            Err(panic_info) => {
                let msg = if let Some(s) = panic_info.downcast_ref::<&str>() {
                    s.to_string()
                } else if let Some(s) = panic_info.downcast_ref::<String>() {
                    s.clone()
                } else {
                    format!("Unknown panic reason for: {}", type_name::<T>())
                };
                Err(Error::other(format!("Factory find failed: {}", msg)))
            }
        }
    }
}

#[macro_export]
macro_rules! use_factory {
    ($t:ty) => {
        Factory::pick::<$t>()
    };
}

#[macro_export]
macro_rules! use_mut_factory {
    ($t:ty) => {
        Factory::pick_mut::<$t>()
    };
}

#[macro_export]
macro_rules! unsafe_factory {
    ($t:ty) => {
        Factory::pick_unsafe::<$t>()
    };
}

#[macro_export]
macro_rules! use_or_register_factory {
    ($t:ty) => {
        Factory::pick_or_register::<$t>()
    };
}
