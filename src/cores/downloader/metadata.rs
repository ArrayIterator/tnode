use std::sync::Arc;
use std::time::Duration;
use reqwest::header::HeaderMap;
use reqwest::Method;
use url::Url;
use crate::cores::downloader::settings::{MAX_MAX_RETRIES, MAX_REDIRECTS, MAX_TIMEOUT, MIN_MAX_RETRIES, MIN_TIMEOUT};
use crate::cores::generator::uuid::Uuid;

#[derive(Debug, Clone)]
pub struct MetaData {
    pub(crate) id: String,
    pub(crate) url: Url,
    pub(crate) headers: Option<Arc<HeaderMap<String>>>,
    pub(crate) filename: Option<String>,
    pub(crate) method: Method,
    pub(crate) follow_redirect: bool,
    pub(crate) timeouts: Duration,
    pub(crate) max_redirects: usize,
    pub(crate) max_retries: usize,
}

impl MetaData {
    pub(crate) fn new<M: Into<Method>>(
        uri: &Url,
        method: M,
        headers: Option<HeaderMap<String>>,
        filename: Option<String>,
        follow_redirect: bool,
        max_redirects: usize,
        max_retries: usize,
        timeouts: Duration,
    ) -> Self {

        Self {
            id: Uuid::v7().to_string(),
            url: uri.clone(),
            headers: headers.as_ref().map(|h| Arc::new(h.clone())),
            method: method.into(),
            filename,
            max_retries: max_retries.clamp(MIN_MAX_RETRIES, MAX_MAX_RETRIES),
            max_redirects: max_redirects.clamp(0, MAX_REDIRECTS),
            timeouts: timeouts.clamp(MIN_TIMEOUT, MAX_TIMEOUT),
            follow_redirect,
        }
    }
    pub fn get_id(&self) -> &str {
        &self.id
    }
    pub fn get_uri(&self) -> &Url {
        &self.url
    }
    pub fn get_headers(&self) -> Option<Arc<HeaderMap<String>>> {
        self.headers.clone()
    }
    pub fn get_filename(&self) -> Option<String> {
        self.filename.clone()
    }
    pub fn get_method(&self) -> &Method {
        &self.method
    }
    pub fn get_follow_redirect(&self) -> bool {
        self.follow_redirect
    }
    pub fn get_timeouts(&self) -> Duration {
        self.timeouts
    }
    pub fn get_max_redirects(&self) -> usize {
        self.max_redirects
    }
    pub fn get_max_retries(&self) -> usize {
        self.max_retries
    }
    pub fn is_retry_supported(&self) -> bool {
        self.max_retries > 0
    }
    pub fn is_retry_limit(&self, retry: usize) -> bool {
        retry >= self.max_retries
    }
    pub fn into_arc(self) -> Arc<Self> {
        Arc::new(self)
    }
}
