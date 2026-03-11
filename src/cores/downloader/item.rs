use std::sync::Arc;
use std::time::Duration;
use reqwest::Method;
use reqwest::header::HeaderMap;
use url::Url;
use crate::cores::downloader::download_manager::DownloadManager;
use crate::cores::downloader::metadata::MetaData;
use crate::cores::downloader::progress::Progress;
use crate::cores::downloader::status::Status;
use crate::cores::net::dns::Dns;
use crate::cores::system::error::{Error, ResultError};

// todo: implement download with hickory
pub type FunctionExecuteProgress = fn(&Progress) -> ResultError<()>;
pub type ArcFunctionExecuteProgress = Arc<FunctionExecuteProgress>;

struct DownloadGuard {
    item: Arc<Item>,
}

impl Drop for DownloadGuard {
    fn drop(&mut self) {
        let mut progress = self.item.get_progress();
        if !progress.is_finished() {
            progress.mark_as_cancelled("Download was dropped without completion or failure");
        }
    }
}

#[derive(Debug, Clone)]
pub struct Item {
    download_manager: Arc<DownloadManager>,
    metadata: Arc<MetaData>,
    progress: Arc<Progress>,
    dns: Option<Arc<Dns>>,
    on_start: Option<ArcFunctionExecuteProgress>,
    on_progress: Option<ArcFunctionExecuteProgress>,
    on_complete: Option<ArcFunctionExecuteProgress>,
    on_fail: Option<ArcFunctionExecuteProgress>,
    on_cancel: Option<ArcFunctionExecuteProgress>,
    on_finally: Option<ArcFunctionExecuteProgress>,
}

impl Item {
    pub(crate) fn new(
        download_manager: Arc<DownloadManager>,
        dns: Option<Arc<Dns>>,
        url: Url,
        method: Method,
        headers: Option<reqwest::header::HeaderMap<String>>,
        filename: Option<String>,
        max_retries: usize,
        max_redirects: usize,
        follow_redirect: bool,
        timeouts: Duration,
        on_start: Option<ArcFunctionExecuteProgress>,
        on_progress: Option<ArcFunctionExecuteProgress>,
        on_complete: Option<ArcFunctionExecuteProgress>,
        on_fail: Option<ArcFunctionExecuteProgress>,
        on_cancel: Option<ArcFunctionExecuteProgress>,
        on_finally: Option<ArcFunctionExecuteProgress>,
    ) -> Self {
        let metadata = Arc::new(MetaData::new(
            &url,
            method,
            headers,
            filename,
            follow_redirect,
            max_redirects,
            max_retries,
            timeouts,
        ));
        Self {
            download_manager: download_manager.clone(),
            metadata: metadata.clone(),
            progress: Progress::use_default(download_manager, metadata).into_arc(),
            dns,
            on_start,
            on_progress,
            on_complete,
            on_fail,
            on_cancel,
            on_finally,
        }
    }
    pub fn get_id(&self) -> &str {
        self.metadata.get_id()
    }
    pub fn get_dns(&self) -> Option<Arc<Dns>> {
        self.dns.clone()
    }
    pub fn get_progress(&self) -> Arc<Progress> {
        self.progress.clone()
    }
    pub fn get_metadata(&self) -> Arc<MetaData> {
        self.metadata.clone()
    }
    pub fn get_max_retries(&self) -> usize {
        self.metadata.get_max_retries()
    }
    pub fn get_max_redirects(&self) -> usize {
        self.metadata.get_max_redirects()
    }
    pub fn get_follow_redirect(&self) -> bool {
        self.metadata.get_follow_redirect()
    }
    pub fn get_timeouts(&self) -> Duration {
        self.metadata.get_timeouts()
    }
    pub fn get_url(&self) -> &Url {
        self.metadata.get_uri()
    }
    pub fn get_method(&self) -> &Method {
        self.metadata.get_method()
    }
    pub fn get_headers(&self) -> Option<Arc<HeaderMap<String>>> {
        self.metadata.get_headers()
    }
    pub fn get_filename(&self) -> Option<String> {
        self.metadata.get_filename()
    }
    pub fn get_download_manager(&self) -> Arc<DownloadManager> {
        self.download_manager.clone()
    }
    pub fn get_status(&self) -> Arc<Status> {
        self.progress.get_status()
    }
    pub fn is_pending(&self) -> bool {
        self.progress.is_pending()
    }
    pub fn is_in_progress(&self) -> bool {
        self.progress.is_in_progress()
    }
    pub fn is_completed(&self) -> bool {
        self.progress.is_completed()
    }
    pub fn is_failed(&self) -> bool {
        self.progress.is_failed()
    }
    pub fn is_cancelled(&self) -> bool {
        self.progress.is_cancelled()
    }
    pub fn is_finished(&self) -> bool {
        self.progress.is_finished()
    }
}

impl Item {
    fn execute_on_start(&self) -> ResultError<()> {
        if let Some(on_start) = &self.on_start {
            on_start(&self.progress)
        } else {
            Ok(())
        }
    }
    fn execute_on_progress(&self) -> ResultError<()> {
        if let Some(on_progress) = &self.on_progress {
            on_progress(&self.progress)
        } else {
            Ok(())
        }
    }
    fn execute_on_complete(&self) -> ResultError<()> {
        if let Some(on_complete) = &self.on_complete {
            on_complete(&self.progress)
        } else {
            Ok(())
        }
    }
    fn execute_on_fail(&self) -> ResultError<()> {
        if let Some(on_fail) = &self.on_fail {
            on_fail(&self.progress)
        } else {
            Ok(())
        }
    }
    fn execute_on_cancel(&self) -> ResultError<()> {
        if let Some(on_cancel) = &self.on_cancel {
            on_cancel(&self.progress)
        } else {
            Ok(())
        }
    }
    fn execute_on_finally(&self) -> ResultError<()> {
        if let Some(on_finally) = &self.on_finally {
            on_finally(&self.progress)
        } else {
            Ok(())
        }
    }
    pub(crate) fn execute<T, Finish, Fut>(self: &Arc<Self>, func: Finish) -> impl Future<Output = ResultError<T>> + Send
    where
        Finish: Fn(&Self) -> Fut + Send + Sync + 'static,
        Fut: Future<Output = ResultError<T>> + 'static + Send,
    {
        async move {
            match self.get_status().as_ref() {
                Status::InProgress => {
                    return Err(Error::invalid_state("Download is already in progress!"));
                },
                Status::Completed => {
                    return Err(Error::invalid_state("Download is already completed!"));
                },
                Status::Cancelled(_) => {
                    return Err(Error::invalid_state("Download is already cancelled!"));
                },
                Status::Failed(error) => {
                    return Err(Error::invalid_state(format!("Download has already failed with error: {}", error)));
                },
                _ => {}
            }
            let dm = self.get_download_manager();
            let id = self.get_id();
            let is_full = !dm.has(id) && dm.is_full();
            if is_full {
                return Err(Error::invalid_state("Download queue is full!"));
            }
            let _guard = DownloadGuard { item: self.clone() };
            match self.execute_on_start() {
                Ok(_) => {},
                Err(e) => {
                    self.get_progress().mark_as_failed(e.clone());
                    return Err(e);
                }
            }
            dm.attach(self.clone());
            let r = func(&self).await;
            r
        }
    }
}
