use std::collections::HashMap;
use std::net::SocketAddr;
use std::ops::Deref;
use std::sync::{Arc, OnceLock};
use std::time::Duration;
use bytes::Bytes;
use reqwest::{ClientBuilder, Method};
use reqwest::header::{HeaderMap, HeaderValue};
use url::Url;
use crate::cores::downloader::download_manager::DownloadManager;
use crate::cores::downloader::metadata::MetaData;
use crate::cores::downloader::progress::Progress;
use crate::cores::downloader::state::State;
use crate::cores::net::dns::{DnsChoice};
use crate::cores::system::error::{ResultError};

pub type FunctionExecuteProgress = fn(&Progress) -> ResultError<()>;
pub type ArcFunctionExecuteProgress = Arc<FunctionExecuteProgress>;

pub type FunctionExcuteDownload = fn(&Progress, usize, Bytes) -> ResultError<bool>;
pub type ArcFunctionExecuteDownload = Arc<FunctionExcuteDownload>;

pub type FunctionClientBuilderFallback = fn(ClientBuilder) -> ResultError<ClientBuilder>;
pub type ArcFunctionClientBuilderFallback = Arc<FunctionClientBuilderFallback>;

// @todo : Implementing

struct DownloadGuard {
    item: Arc<Item>,
}

impl Drop for DownloadGuard {
    fn drop(&mut self) {
        if let Some(state) = self.item.state.get() {
            if !state.is_finished() {
                state.cancel("Download was dropped without completion or failure").ok();
            }
        }
    }
}

#[derive(Debug, Clone)]
pub struct Item {
    pub(crate) download_manager: Arc<DownloadManager>,
    pub(crate) metadata: Arc<MetaData>,
    pub(crate) dns: Option<Arc<DnsChoice>>,
    pub(crate) builder_fallback: Option<ArcFunctionClientBuilderFallback>,
    pub(crate) on_start: Option<ArcFunctionExecuteProgress>,
    pub(crate) on_progress: Option<ArcFunctionExecuteProgress>,
    pub(crate) on_download_progress: Option<ArcFunctionExecuteDownload>,
    pub(crate) on_complete: Option<ArcFunctionExecuteProgress>,
    pub(crate) on_fail: Option<ArcFunctionExecuteProgress>,
    pub(crate) on_cancel: Option<ArcFunctionExecuteProgress>,
    pub(crate) on_finish: Option<ArcFunctionExecuteProgress>,
    pub(crate) state: Arc<OnceLock<Arc<State>>>,
}

impl Item {
    pub(crate) fn new<D: Into<Arc<DnsChoice>>>(
        download_manager: Arc<DownloadManager>,
        dns: Option<D>,
        url: Url,
        method: Method,
        headers: Option<HeaderMap<HeaderValue>>,
        filename: Option<String>,
        max_retries: usize,
        max_redirects: usize,
        follow_redirect: bool,
        timeout: Duration,
        connect_timeout: Duration,
        insecure: bool,
        domain_resolve: Option<HashMap<String, Vec<SocketAddr>>>,
        on_start: Option<ArcFunctionExecuteProgress>,
        on_progress: Option<ArcFunctionExecuteProgress>,
        on_download_progress: Option<ArcFunctionExecuteDownload>,
        on_complete: Option<ArcFunctionExecuteProgress>,
        on_fail: Option<ArcFunctionExecuteProgress>,
        on_cancel: Option<ArcFunctionExecuteProgress>,
        on_finish: Option<ArcFunctionExecuteProgress>,
        builder_fallback: Option<Arc<FunctionClientBuilderFallback>>,
    ) -> Self where Self: Sized {
        let metadata = Arc::new(MetaData::new(
            &url,
            method,
            headers,
            filename,
            follow_redirect,
            max_redirects,
            max_retries,
            timeout,
            connect_timeout,
            insecure,
            domain_resolve
        ));
        Self {
            download_manager: download_manager.clone(),
            metadata: metadata.clone(),
            dns: dns.map(|e|e.into()),
            on_start,
            on_progress,
            on_download_progress,
            on_complete,
            on_fail,
            on_cancel,
            on_finish,
            builder_fallback,
            state: Arc::new(OnceLock::new()),
        }
    }
    pub fn get_metadata(&self) -> Arc<MetaData> {
        self.metadata.clone()
    }
    pub fn get_dns(&self) -> Option<Arc<DnsChoice>> {
        self.dns.clone()
    }
    pub fn get_id(&self) -> &str {
        self.metadata.get_id()
    }
    pub fn get_download_manager(&self) -> Arc<DownloadManager> {
        self.download_manager.clone()
    }
    pub fn get_state(self: &Arc<Self>) -> Arc<State> {
        self.state.get_or_init(|| Arc::new(State::new(self.clone()))).clone()
    }
}

impl Deref for Item {
    type Target = MetaData;
    fn deref(&self) -> &Self::Target {
        &self.metadata
    }
}

// impl Item {
//     fn execute_on_start(&self) -> ResultError<()> {
//         if let Some(on_start) = &self.on_start {
//             on_start(&self.progress)
//         } else {
//             Ok(())
//         }
//     }
//     fn execute_on_progress(&self) -> ResultError<()> {
//         if let Some(on_progress) = &self.on_progress {
//             on_progress(&self.progress)
//         } else {
//             Ok(())
//         }
//     }
//     fn execute_on_download_progress(&self, downloaded: usize, chunk: Bytes) -> ResultError<bool> {
//         if let Some(on_download_progress) = &self.on_download_progress {
//             on_download_progress(&self.progress, downloaded, chunk)
//         } else {
//             Ok(true)
//         }
//     }
//     fn execute_on_complete(&self) -> ResultError<()> {
//         if let Some(on_complete) = &self.on_complete {
//             on_complete(&self.progress)
//         } else {
//             Ok(())
//         }
//     }
//     fn execute_on_fail(&self) -> ResultError<()> {
//         if let Some(on_fail) = &self.on_fail {
//             on_fail(&self.progress)
//         } else {
//             Ok(())
//         }
//     }
//     fn execute_on_cancel(&self) -> ResultError<()> {
//         if let Some(on_cancel) = &self.on_cancel {
//             on_cancel(&self.progress)
//         } else {
//             Ok(())
//         }
//     }
//     fn execute_on_finally(&self) -> ResultError<()> {
//         if self.finally_dispatched.swap(true, Ordering::SeqCst) {
//             return Ok(()); // already dispatched, just return ok
//         }
//         if let Some(on_finally) = &self.on_finish {
//             on_finally(&self.progress)
//         } else {
//             Ok(())
//         }
//     }
//     pub fn cancel(&self, reason: Option<String>) -> ResultError<()> {
//         if self.atomic_cancel.load(Ordering::SeqCst) {
//             return Err(Error::invalid_state("Download is already cancelled!"));
//         }
//         let mut progress = self.get_progress();
//         match progress.get_status().as_ref() {
//             Status::Cancelled(_) => {
//                 return Err(Error::invalid_state("Download is already cancelled!"));
//             },
//             Status::Failed(error) => {
//                 return Err(Error::invalid_state(format!("Download has already failed with error: {}", error)));
//             },
//             Status::Completed => {
//                 return Err(Error::invalid_state("Download is already completed!"));
//             },
//             _ => {
//                 progress.mark_as_cancelled(reason.unwrap_or_else(|| "Cancelled by user".to_string()));
//                 self.execute_on_cancel()?;
//                 self.execute_on_finally()?;
//                 return Ok(());
//             },
//         }
//     }
//     pub fn retry(&self) -> ResultError<()> {
//         let mut progress = self.get_progress();
//         if !progress.is_failed() {
//             return Err(Error::invalid_state("Download is not in a failed state!"));
//         }
//         progress.reset(true);
//         self.execute_on_progress()?;
//         Ok(())
//     }
//     pub fn has_final_result(&self) -> bool {
//         self.final_res.read().is_some()
//     }
//     pub fn get_final_result(&self) -> Option<Result<Arc<dyn Any + Send + Sync>, Arc<Error>>> {
//         let final_res = self.final_res.read();
//         final_res.clone()
//     }
// }
