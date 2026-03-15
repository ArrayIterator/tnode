use std::{fmt::Display, sync::{Arc, LazyLock, atomic::AtomicUsize}};
use parking_lot::{RwLock};
use reqwest::{Client, ClientBuilder, header::{self, HeaderMap, HeaderValue}, redirect, retry::{for_host, never}};
use crate::cores::{downloader::item::Item, system::{error::{Error, ResultError}, runtime::Runtime}};


pub const DEFAULT_HEADER: LazyLock<HeaderMap<HeaderValue>> = LazyLock::new(|| {
    let mut headers = HeaderMap::new();
    headers.insert(header::USER_AGENT, HeaderValue::from_static(Runtime::app_default_user_agent()));
    headers.insert(
        header::ACCEPT,
        HeaderValue::from_static("text/html,application/xhtml+xml,application/xml;q=0.9,*/*;q=0.8")
    );
    headers.insert(
        reqwest::header::ACCEPT_ENCODING,
        HeaderValue::from_static("gzip, deflate, br")
    );
    headers.insert(
        reqwest::header::RANGE,
        HeaderValue::from_static("bytes=0-")
    );
    headers
});


#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub enum Status {
    #[default]
    Pending,
    Downloading,
    Paused,
    Completed,
    Failed(String),
    Canceled(String),
}

impl Status {
    pub fn is_finished(&self) -> bool {
        matches!(self, Status::Completed | Status::Failed(_) | Status::Canceled(_))
    }
    pub fn is_in_progress(&self) -> bool {
        matches!(self, Status::Downloading)
    }
    pub fn is_pending(&self) -> bool {
        matches!(self, Status::Pending)
    }
    pub fn is_paused(&self) -> bool {
        matches!(self, Status::Paused)
    }
    pub fn is_failed(&self) -> bool {
        matches!(self, Status::Failed(_))
    }
    pub fn is_canceled(&self) -> bool {
        matches!(self, Status::Canceled(_))
    }
    pub fn is_completed(&self) -> bool {
        matches!(self, Status::Completed)
    }
    pub fn as_name(&self) -> String {
        match self {
            Status::Pending => "Pending".to_string(),
            Status::Downloading => "Downloading".to_string(),
            Status::Paused => "Paused".to_string(),
            Status::Completed => "Completed".to_string(),
            Status::Failed(e) => format!("Failed({})", e),
            Status::Canceled(e) => format!("Canceled({})", e),
        }
    }
    pub fn can_transition_to(&self, new_state: &Status) -> bool {
        match self {
            Status::Pending => matches!(new_state, Status::Downloading | Status::Paused | Status::Canceled(_)),
            Status::Downloading => matches!(new_state, Status::Paused | Status::Completed | Status::Failed(_) | Status::Canceled(_)),
            Status::Paused => matches!(new_state, Status::Downloading | Status::Canceled(_) | Status::Failed(_) | Status::Completed),
            e => matches!(new_state, Status::Failed(_) | Status::Canceled(_) | Status::Completed),
        }
    }
}

impl Display for Status {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.as_name())
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub enum ResumableState {
    #[default]
    Unknown,
    Resumable,
    NonResumable,
}

#[derive(Debug, Clone, Default)]
pub(crate) struct StateRecord {
    pub(crate) resumable: Arc<RwLock<ResumableState>>,
    pub(crate) status: Arc<RwLock<Status>>,
    pub(crate) previous_status: Arc<RwLock<Status>>,
    // store the start time as a unix timestamp in seconds, using AtomicUsize for thread-safe read/write without needing a mutex, and it can represent a wide range of dates (up to the year 2554)
    // if the value is 0, it means the start time has not been set yet
    pub(crate) start_time: Arc<AtomicUsize>,
    pub(crate) fail_time: Arc<AtomicUsize>, // same as start_time,
    pub(crate) duration: Arc<AtomicUsize>, // store the duration in seconds, using AtomicUsize for thread-safe read/write without needing a mutex, and it can represent durations of up to approximately 136 years
}

impl StateRecord {
    fn set_status(&self, new_status: Status) -> ResultError<()> {
        let mut status = self.status.write();
        if !status.can_transition_to(&new_status) {
            return Err(Error::invalid_state(format!("Cannot transition from {:?} to {:?}", *status, new_status)));
        }
        *self.previous_status.write() = status.clone();
        *status = new_status;
        Ok(())
    }
    pub fn get_status(&self) -> Status {
        self.status.read().clone()
    }
    pub fn get_previous_status(&self) -> Status {
        self.previous_status.read().clone()
    }
}

#[derive(Debug, Clone)]
pub struct State {
    pub(crate) item: Arc<Item>,
    pub(crate) record: Arc<StateRecord>,
    pub(crate) client: Arc<RwLock<Option<Client>>>,
}

impl State {
    pub(crate) fn new(item: Arc<Item>) -> Self {
        Self {
            item,
            record: Arc::new(StateRecord::default()),
            client: Arc::new(RwLock::new(None)),
        }
    }

    pub(crate) fn client(&self) -> ResultError<Client> {
        if let Some(c) = self.client.read().as_ref() {
            return Ok(c.clone());
        }
        let client = self.builder().build().map_err(Error::from_error)?;
        *self.client.write() = Some(client.clone());
        Ok(client)
    }

    pub(crate) fn builder(&self) -> ClientBuilder {
        let item = self.get_item();
        let headers = item.get_headers().map(|e|{
            let mut headers = e.as_ref().clone();
            for (name, value) in DEFAULT_HEADER.iter() {
                headers.entry(name).or_insert_with(|| value.clone());
            }
            headers
        }).unwrap_or_else(||DEFAULT_HEADER.clone());
        let max_redirect = item.get_max_redirects();
        let redirect_policy = if item.get_follow_redirect() && max_redirect > 0 {
            redirect::Policy::limited(max_redirect)
        } else {
            redirect::Policy::none()
        };
        let max_retry = item.get_max_retries();
        let retry_policy = if max_retry > 0 && let Some(host) = item.get_url().host() {
            for_host(host.to_string())
                .max_retries_per_request(max_retry as u32)
                .classify_fn(|req|req.retryable())
        } else {
            never()
        };
        let mut builder = Client::builder()
            .default_headers(headers)
            .redirect(redirect_policy)
            .retry(retry_policy)
            .timeout(item.get_timeout())
            .connect_timeout(item.get_connect_timeout())
            .danger_accept_invalid_certs(item.is_insecure())
            .tcp_nodelay(true)
            .tls_backend_rustls();
        if let Some(dns) = item.get_dns() {
            builder = builder.dns_resolver(dns.clone()).hickory_dns(false);
        }
        let resolved_addrs = item.get_metadata().get_domain_resolve();
        if let Some(domain_resolve) = resolved_addrs {
            for (domain, addrs) in domain_resolve.iter() {
                if addrs.is_empty() {
                    continue;
                }
                builder = builder.resolve_to_addrs(domain, &addrs);
            }
        }
        builder
    }
}

impl State {
    pub(crate) fn set_status(&self, new_status: Status) -> ResultError<()> {
        self.record.set_status(new_status)
    }
}

impl State {
    pub fn get_item(&self) -> Arc<Item> {
        self.item.clone()
    }
    pub fn get_record(&self) -> Arc<StateRecord> {
        self.record.clone()
    }
    pub fn cancel<E: Into<Error>>(&self, error: E) -> ResultError<()> {
        let error = error.into();
        self.set_status(Status::Canceled(error.to_string()))?;
        Ok(())
    }
    pub fn is_finished(&self) -> bool {
        self.record.get_status().is_finished()
    }
}

/*

pub(crate) fn execute<T, Finish, Fut>(self: &Arc<Self>, func: Finish) -> impl Future<Output = Result<Arc<T>, Arc<Error>>> + Send
where
    T: Any + Send + Sync + 'static,
    Finish: Fn(&Self) -> Fut + 'static + Send + Sync,
    Fut: Future<Output = ResultError<T>> + 'static + Send + Sync,
{
    async move {
        match (match self.get_status().as_ref() {
            Status::InProgress(_) => Err(Error::invalid_state("Download is already in progress!")),
            Status::Completed => Err(Error::invalid_state("Download is already completed!")),
            Status::Cancelled(_) => Err(Error::invalid_state("Download is already cancelled!")),
            Status::Failed(error) => Err(Error::invalid_state(format!("Download has already failed with error: {}", error))),
            _ => Ok(())
        }.map_err(Arc::new)) {
            Err(e) => {
                return Err(e);
            },
            Ok(_) => {},
        }

        let dm = self.get_download_manager();
        let id = self.get_id();
        let is_full = !dm.has(id) && dm.is_full();
        if is_full {
            return Err(Arc::new(Error::invalid_state("Download queue is full!")));
        }
        let _guard = DownloadGuard { item: self.clone() };
        let res: Result<Arc<T>, Arc<Error>> = async move {
            self.execute_on_start().map_err(|e|{
                let e = Arc::new(e);
                self.get_progress().mark_as_failed(e.clone());
                e
            })?;
            dm.attach(self.clone());
            let headers = self.get_headers().map(|e|{
                let mut headers = e.as_ref().clone();
                for (name, value) in DEFAULT_HEADER.iter() {
                    headers.entry(name).or_insert_with(|| value.clone());
                }
                headers
            }).unwrap_or(DEFAULT_HEADER.clone());
            let max_redirect = self.get_max_redirects();
            let redirect_policy = if self.get_follow_redirect() && max_redirect > 0 {
                redirect::Policy::limited(max_redirect)
            } else {
                redirect::Policy::none()
            };
            let max_retry = self.get_max_retries();
            let retry_policy = if max_retry > 0 && let Some(host) = self.get_url().host() {
                for_host(host.to_string())
                    .max_retries_per_request(max_retry as u32)
                    .classify_fn(|req|req.retryable())
            } else {
                never()
            };
            let mut builder = Client::builder()
                .default_headers(headers)
                .redirect(redirect_policy)
                .retry(retry_policy)
                .timeout(self.get_timeout())
                .connect_timeout(self.get_connect_timeout())
                .danger_accept_invalid_certs(self.is_insecure())
                .tcp_nodelay(true)
                .tls_backend_rustls();
            if let Some(dns) = self.get_dns() {
                builder = builder.dns_resolver(dns).hickory_dns(false);
            }
            let resolved_addrs = self.get_metadata().get_domain_resolve();
            if let Some(domain_resolve) = resolved_addrs {
                for (domain, addrs) in domain_resolve.iter() {
                    if addrs.is_empty() {
                        continue;
                    }
                    builder = builder.resolve_to_addrs(domain, &addrs);
                }
            }
            builder = if let Some(builder_fallback) = &self.builder_fallback {
                builder_fallback(builder)?
            } else {
                builder
            };
            let client = builder
                .build()
                .map_err(Error::from_error)?;
            let url = self.get_url().clone();
            let method = self.get_method().clone();
            self.execute_on_start()?;
            let response = client
                .request(method, url)
                .send()
                .await
                .map_err(|e| {
                    let err = Arc::new(Error::from_error(e));
                    self.get_progress().mark_as_failed(err.clone());
                    self.execute_on_fail().ok();
                    err
                })?;
            let total_size = response
                .headers()
                .get(reqwest::header::CONTENT_LENGTH)
                .and_then(|val| val.to_str().ok())
                .and_then(|s| s.parse::<isize>().ok())
                .unwrap_or(-1);
            let progress = self.get_progress();

            progress.set_known_size(total_size);
            let mut stream = response.bytes_stream(); // todo: completing the download and writing to file, and updating progress accordingly
            stream.inspect(|res_bytes| {
                match res_bytes {
                    Ok(bytes) => {
                        let len = bytes.len();
                        progress.set_downloaded(len);
                        self.execute_on_download_progress(
                            len,
                            bytes.clone()
                        ).ok();
                    },
                    Err(e) => {
                        let err = Arc::new(Error::resource_unavailable(format!("Failed to read response stream: {}", e)));
                        progress.mark_as_failed(err.clone());
                        self.execute_on_fail().ok();
                    },
                }
            });

            // detach is done in the guard's drop, so that if the future is dropped before completion, it will automatically detach from the download manager and mark as cancelled
            let res = func(&self)
                .await
                .map(|e| {
                    let res = Arc::new(e);
                    self.final_res.write().replace(Ok(res.clone()));
                    res
                })
                .map_err(|e| {
                    let e = Arc::new(e);
                    self.final_res.write().replace(Err(e.clone()));
                    e
                });
            dm.detach(id);
            Ok(res?)
        }.await;
        Ok(res?)
    }
}
 */
