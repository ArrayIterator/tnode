use dashmap::DashMap;
use std::any::{type_name, Any, TypeId};
use std::fmt::Debug;
use std::io::{Error, ErrorKind};
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;
use std::time::Duration;
use tokio::sync::broadcast::error::{RecvError, SendError};
use tokio::sync::broadcast::{channel, Receiver, Sender};

pub const MAX_CHANNELS_CAPACITY: usize = 8192;
pub const MAX_CHANNEL_CAPACITY: usize = 8192;
pub const MIN_CHANNEL_CAPACITY: usize = 8;
pub const MIN_CHANNELS_CAPACITY: usize = 8;

pub type BroadCastSendResult<S, T> = Result<S, SendError<T>>;
pub type BroadReceiveResult<T> = Result<T, RecvError>;
pub type OperationError<T> = Result<T, Error>;

pub trait EventSender: Send + Sync + Debug {
    fn receiver_count(&self) -> usize;
    fn is_empty(&self) -> bool;
    fn as_any(&self) -> &dyn Any;
    fn as_any_mut(&mut self) -> &mut dyn Any;
}

impl<T: Clone + Send + Sync + 'static> EventSender for Sender<T> {
    fn receiver_count(&self) -> usize {
        Sender::receiver_count(self)
    }
    fn is_empty(&self) -> bool {
        Sender::is_empty(self)
    }
    fn as_any(&self) -> &dyn Any {
        self
    }
    fn as_any_mut(&mut self) -> &mut dyn Any {
        self
    }
}

#[derive(Debug)]
pub struct Channel {
    broadcaster: Box<dyn EventSender>,
    type_id: TypeId,
    capacity: usize,
}

impl Channel {
    pub fn new<T: Clone + Send + Sync + 'static>(capacity: usize) -> Self {
        let capacity = capacity.clamp(MIN_CHANNEL_CAPACITY, MAX_CHANNEL_CAPACITY);
        let (tx, _) = channel::<T>(capacity);
        Self {
            broadcaster: Box::new(tx),
            type_id: TypeId::of::<T>(),
            capacity,
        }
    }
    pub fn subscribe<T: Clone + Send + Sync + 'static>(&self) -> OperationError<Receiver<T>> {
        match self.broadcaster.as_any().downcast_ref::<Sender<T>>() {
            None => Err(Error::new(
                ErrorKind::Other,
                format!(
                    "Can not downcast reference subscriber:  {:?}",
                    type_name::<T>()
                ),
            )),
            Some(sender) => Ok(sender.subscribe()),
        }
    }
    pub fn send<T: Clone + Send + Sync + 'static>(&self, data: T) -> BroadCastSendResult<usize, T> {
        match self.broadcaster.as_any().downcast_ref::<Sender<T>>() {
            None => Err(SendError(data)),
            Some(sender) => sender.send(data),
        }
     }
     pub fn capacity(&self) -> usize {
        self.capacity
     }
}

impl EventSender for Channel {
    fn receiver_count(&self) -> usize {
        self.broadcaster.receiver_count()
    }
    fn is_empty(&self) -> bool {
        self.broadcaster.is_empty()
    }
    fn as_any(&self) -> &dyn Any {
        self.broadcaster.as_any()
    }
    fn as_any_mut(&mut self) -> &mut dyn Any {
        self.broadcaster.as_any_mut()
    }
}

#[derive(Debug, Clone)]
pub struct EventManager {
    channels: Arc<DashMap<TypeId, Channel>>,
    capacity: usize,
    default_capacity: usize,
    task_running: Arc<AtomicBool>,
}

impl EventManager {
    pub const DEFAULT_CHANNEL_CAPACITY: usize = 128;
    pub const DEFAULT_CHANNELS_CAPACITY: usize = 2048;

    pub fn new(capacity: usize, default_capacity: usize) -> Self {
        let capacity = capacity.clamp(MIN_CHANNELS_CAPACITY, MAX_CHANNELS_CAPACITY);
        Self {
            capacity,
            channels: Arc::new(DashMap::with_capacity(capacity)),
            default_capacity: default_capacity.clamp(MIN_CHANNEL_CAPACITY, MAX_CHANNEL_CAPACITY),
            task_running: Arc::new(AtomicBool::new(false)),
        }
    }

    fn retain_capacity(&self) -> bool {
        let channel_count = self.channels.len();
        if channel_count < self.capacity {
            return true;
        }
        let type_id_empty = self.channels.iter().find(|entry| {
            let entry = entry.value();
            entry.receiver_count() == 0
        }).map(|entry| *entry.key());
        // remove first capacity if capacity is reached
        if let Some(type_id) = type_id_empty {
            self.channels.remove(&type_id);
            return true;
        }
        false
    }

    fn __auto_recycle(&self) {
        if self.task_running.swap(true, Ordering::SeqCst) {
            return;
        }
        let channels = Arc::clone(&self.channels);
        let task_running = Arc::clone(&self.task_running);
        tokio::spawn(async move {
            let mut interval = tokio::time::interval(Duration::from_secs(300));
            loop {
                interval.tick().await;
                channels.retain(|_name, channel| channel.receiver_count() > 0);

                if channels.is_empty() {
                    task_running.store(false, Ordering::SeqCst);
                    if !channels.is_empty() {
                        if !task_running.swap(true, Ordering::SeqCst) {
                            continue;
                        }
                    }
                    break;
                }
            }
        });
    }

    fn remove_by_type_id(&self, type_id: &TypeId) -> Option<Channel> {
        self.channels.remove(type_id).map(|e| e.1)
    }

    pub fn remove<T: Send + Sync + 'static>(&self) -> Option<Channel> {
        let type_id = TypeId::of::<T>();
        self.remove_by_type_id(&type_id)
    }

    pub fn has<T: Send + Sync + 'static>(&self) -> bool {
        self.channels.contains_key(&TypeId::of::<T>())
    }

    pub fn subscribe_with_capacity<T: Clone + Send + Sync + 'static>(&self, capacity: usize) -> OperationError<Receiver<T>> {
        if !self.retain_capacity() {
            return Err(Error::new(
                ErrorKind::Other,
                format!(
                    "Channel capacity reached: {}. Consider increasing the capacity or removing unused channels.",
                    self.capacity
                ),
            ));
        }
        let capacity = capacity.clamp(MIN_CHANNEL_CAPACITY, MAX_CHANNEL_CAPACITY);
        let type_id = TypeId::of::<T>();
        let entry = self.channels.entry(type_id).or_insert_with(|| {
            Channel::new::<T>(capacity)
        });
        match entry.as_any().downcast_ref::<Sender<T>>() {
            None => Err(Error::new(
                ErrorKind::Other,
                format!(
                    "Can not downcast reference subscriber:  {:?}",
                    type_name::<T>()
                ),
            )),
            Some(sender) => Ok(sender.subscribe()),
        }
    }

    pub fn subscribe<T: Clone + Send + Sync + 'static>(&self) -> OperationError<Receiver<T>> {
        self.subscribe_with_capacity::<T>(self.default_capacity)
    }

    pub fn emit<T: Clone + Send + Sync + 'static>(&self, data: T) -> BroadCastSendResult<usize, T> {
        let type_id = TypeId::of::<T>();
        if let Some(entry) = self.channels.get(&type_id) {
            if let Some(sender) = entry.as_any().downcast_ref::<Sender<T>>() {
                return sender.send(data);
            }
        }
        Ok(0)
    }

    pub fn channels_names(&self) -> Vec<TypeId> {
        self.channels.iter().map(|e| e.key().clone()).collect()
    }

    pub fn recycle(&self) -> Vec<TypeId> {
        let mut recycled: Vec<TypeId> = Vec::new();
        self.channels.retain(|name, channel_any| {
            if channel_any.receiver_count() == 0 {
                recycled.push(*name);
                false
            } else {
                true
            }
        });
        recycled
    }
}

impl Default for EventManager {
    fn default() -> Self {
        Self::new(
            Self::DEFAULT_CHANNELS_CAPACITY,
            Self::DEFAULT_CHANNEL_CAPACITY
        )
    }
}
