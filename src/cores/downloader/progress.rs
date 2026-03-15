use std::sync::Arc;
use std::sync::atomic::{AtomicBool, AtomicUsize, Ordering};
use std::time::Duration;
use chrono::{DateTime, Timelike, Utc};
use parking_lot::RwLock;
use crate::cores::downloader::download_manager::DownloadManager;
use crate::cores::downloader::metadata::MetaData;
use crate::cores::downloader::status::Status;
use crate::cores::system::error::{Error, ResultError};

#[derive(Debug, Clone)]
pub struct Progress {
    pub(crate) metadata: Arc<MetaData>,
    pub(crate) status: Arc<RwLock<Arc<Status>>>,
    pub(crate) start_time: Arc<RwLock<Option<Arc<DateTime<Utc>>>>>,
    pub(crate) fail_time: Arc<RwLock<Option<Arc<DateTime<Utc>>>>>,
    pub(crate) duration: Arc<RwLock<Option<Arc<Duration>>>>,
    pub(crate) download_manager: Arc<DownloadManager>,
    pub(crate) downloaded: Arc<AtomicUsize>,
    pub(crate) known_size: Arc<AtomicUsize>,
    pub(crate) retry_count: Arc<AtomicUsize>,
    pub(crate) has_known_size: Arc<AtomicBool>,
}

impl Progress {
    pub(crate) fn use_default(
        download_manager: Arc<DownloadManager>,
        metadata: Arc<MetaData>,
    ) -> Self {
        Self {
            start_time: Arc::new(RwLock::new(None)),
            fail_time: Arc::new(RwLock::new(None)),
            duration: Arc::new(RwLock::new(None)),
            downloaded: Arc::new(AtomicUsize::new(0)),
            known_size: Arc::new(AtomicUsize::new(0)),
            retry_count: Arc::new(AtomicUsize::new(0)),
            has_known_size: Arc::new(AtomicBool::new(false)),
            status: Arc::new(RwLock::new(Status::Pending.into_arc())),
            download_manager,
            metadata,
        }
    }
    pub(crate) fn into_arc(self) -> Arc<Self> {
        Arc::new(self)
    }
}

impl Progress {
    pub fn get_metadata(&self) -> Arc<MetaData> {
        self.metadata.clone()
    }
    pub fn get_downloaded(&self) -> usize {
        self.downloaded.load(Ordering::SeqCst)
    }
    pub fn get_known_size(&self) -> usize {
        self.known_size.load(Ordering::SeqCst)
    }
    pub fn get_retry_count(&self) -> usize {
        self.retry_count.load(Ordering::SeqCst)
    }
    pub fn has_known_size(&self) -> bool {
        self.has_known_size.load(Ordering::SeqCst)
    }
    pub fn get_progress_percentage(&self) -> f64 {
        let known_size = self.get_known_size();
        if known_size <= 0 {
            0.0
        } else {
            (self.get_downloaded() as f64 / known_size as f64) * 100.0
        }
    }
    pub fn get_status(&self) -> Arc<Status> {
        let arc = self.status.read().clone();
        arc
    }
    pub fn get_download_manager(&self) -> Arc<DownloadManager> {
        self.download_manager.clone()
    }
    pub fn get_start_time(&self) -> Option<Arc<DateTime<Utc>>> {
        self.start_time.read().clone()
    }
    pub fn get_duration(&self) -> Option<Arc<Duration>> {
        self.duration.read().clone()
    }
    pub fn get_fail_time(&self) -> Option<Arc<DateTime<Utc>>> {
        self.fail_time.read().clone()
    }
    pub fn is_pending(&self) -> bool {
        self.get_status().is_pending()
    }
    pub fn is_in_progress(&self) -> bool {
        self.get_status().is_in_progress()
    }
    pub fn is_completed(&self) -> bool {
        self.get_status().is_completed()
    }
    pub fn is_failed(&self) -> bool {
        self.get_status().is_failed()
    }
    pub fn is_cancelled(&self) -> bool {
        self.get_status().is_cancelled()
    }
    pub fn is_finished(&self) -> bool {
        self.get_status().is_finished()
    }
    pub(crate) fn mark_final_duration(&self, force: bool) {
        if !self.is_pending() {
            if force || self.get_duration().is_none() {
                let mut duration = 0;
                if let Some(start_time) = &self.get_start_time() {
                    duration = Utc::now().nanosecond() - start_time.nanosecond();
                    *self.duration.write() = Some(Arc::new(Duration::from_nanos(duration as u64)));
                }
            }
        }
    }
    pub(crate) fn mark_as(&self, status: Status) -> &Self {
        // internal method to update the status of the download
        if self.is_pending() && status.is_in_progress() {
            *self.start_time.write() = Some(Arc::new(Utc::now()));
            *self.status.write() = status.into_arc();
        } else {
            match status {
                Status::Pending|
                Status::InProgress(_) => {
                    *self.status.write() = status.into_arc();
                }
                _ => {
                    // Detach the progress from the download manager to prevent memory leaks
                    self.download_manager.detach(self.metadata.get_id());
                    self.mark_final_duration(false);
                    let prev_status = self.status.clone();
                    *self.status.write() = status.into_arc();
                    if !self.is_completed() && self.get_fail_time().is_none() {
                        *self.fail_time.write() = Some(Arc::new(Utc::now()));
                    }
                }
            };
        }
        self
    }

    pub fn get_current_duration(&self) -> Arc<Duration> {
        match self.get_duration() {
            Some(d) => d.clone(),
            None => {
                match self.get_start_time() {
                    None => Arc::new(Duration::from_secs(0)), // empty
                    Some(start_time) => {
                        Arc::new(Duration::from_nanos((Utc::now().nanosecond() - start_time.nanosecond()) as u64))
                    }
                }
            }
        }
    }

    pub fn is_oversize(&self) -> bool {
        if !self.has_known_size() {
            return false;
        }
        let known_size = self.get_known_size();
        if known_size <= 0 {
            return false;
        }
        self.get_downloaded() > known_size
    }
    pub fn is_undownloaded(&self) -> bool {
        self.is_pending() || self.get_downloaded() == 0
    }
    pub(crate) fn mark_as_started(&self) -> &Self {
        self.mark_as(Status::InProgress("Download started".to_string()))
    }
    pub(crate) fn mark_progress<T: Into<String>>(&self, message: T) -> &Self {
        self.mark_as(Status::InProgress(message.into()))
    }
    pub(crate) fn mark_as_completed(&self) -> &Self {
        self.mark_as(Status::Completed)
    }
    pub(crate) fn mark_as_cancelled<T: AsRef<str>>(&self, reason: T) -> &Self {
        self.mark_as(Status::Cancelled(reason.as_ref().to_string()))
    }
    pub(crate) fn mark_as_failed(&self, error: Arc<Error>) -> &Self {
        self.mark_as(Status::Failed(error))
    }
    pub(crate) fn set_known_size(&self, size: isize) -> &Self {
        if self.is_pending() {
            return self;
        }
        if size < 0 {
            self.has_known_size.store(false, Ordering::SeqCst);
            self.known_size.store(0, Ordering::SeqCst);
            return self;
        }
        let size = size as usize;
        self.known_size.store(size, Ordering::SeqCst);
        self.has_known_size.store(true, Ordering::SeqCst);
        self
    }

    pub(crate) fn set_downloaded(&self, size: usize) -> &Self {
        if self.is_pending() {
            return self;
        }
        self.downloaded.fetch_add(size, Ordering::SeqCst);
        self
    }

    pub fn is_retryable(&self) -> bool {
        if self.is_finished() {
            return false;
        }
        let retry_count = self.get_retry_count() + 1; // increment retry count by
        self.get_metadata().is_retry_limit(retry_count)
    }

    pub(crate) fn reset(&self, as_retry: bool) -> ResultError<&Self> {
        *self.start_time.write() = None;
        *self.fail_time.write() = None;
        *self.duration.write() = None;
        *self.status.write() = Status::Pending.into_arc();
        self.downloaded.store(0, Ordering::SeqCst);
        self.known_size.store(0, Ordering::SeqCst);
        if as_retry {
            let retry_count = self.get_retry_count() + 1; // increment retry count by 1
            if self.get_metadata().is_retry_limit(retry_count) {
                return Err(Error::invalid_state(format!("Retry limit of {} exceeded for download with id {}", retry_count, self.get_metadata().get_id())));
            }
            self.retry_count.store(retry_count, Ordering::SeqCst);
        } else {
            // reset retry count to 0 if not a retry
            self.retry_count.store(0, Ordering::SeqCst);
        }
        Ok(self)
    }
}
