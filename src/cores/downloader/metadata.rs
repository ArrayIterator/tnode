use std::collections::HashMap;
use std::net::SocketAddr;
use std::sync::Arc;
use std::time::Duration;
use reqwest::header::{HeaderMap, HeaderValue};
use reqwest::Method;
use url::Url;
use crate::cores::downloader::settings::{MAX_CONNECT_TIMEOUT, MAX_MAX_RETRIES, MAX_REDIRECTS, MAX_TIMEOUT, MIN_CONNECT_TIMEOUT, MIN_MAX_RETRIES, MIN_TIMEOUT};
use crate::cores::generator::uuid::Uuid;

#[derive(Debug, Clone)]
pub struct MetaData {
    pub(crate) id: String,
    pub(crate) url: Url,
    pub(crate) headers: Option<Arc<HeaderMap<HeaderValue>>>,
    pub(crate) filename: Option<String>,
    pub(crate) method: Method,
    pub(crate) follow_redirect: bool,
    pub(crate) timeout: Duration,
    pub(crate) connect_timeout: Duration,
    pub(crate) max_redirects: usize,
    pub(crate) max_retries: usize,
    pub(crate) insecure: bool,
    pub(crate) domain_resolve: Option<Arc<HashMap<String, Vec<SocketAddr>>>>,
}

impl MetaData {
    pub(crate) fn new<M: Into<Method>>(
        uri: &Url,
        method: M,
        headers: Option<HeaderMap<HeaderValue>>,
        filename: Option<String>,
        follow_redirect: bool,
        max_redirects: usize,
        max_retries: usize,
        timeout: Duration,
        connect_timeout: Duration,
        insecure: bool,
        domain_resolve: Option<HashMap<String, Vec<SocketAddr>>>,
    ) -> Self {
        Self {
            id: Uuid::v7().to_string(),
            url: uri.clone(),
            headers: headers.as_ref().map(|h| Arc::new(h.clone())),
            method: method.into(),
            filename,
            max_retries: max_retries.clamp(MIN_MAX_RETRIES, MAX_MAX_RETRIES),
            max_redirects: max_redirects.clamp(0, MAX_REDIRECTS),
            timeout: timeout.clamp(MIN_TIMEOUT, MAX_TIMEOUT),
            connect_timeout: connect_timeout.clamp(MIN_CONNECT_TIMEOUT, MAX_CONNECT_TIMEOUT),
            follow_redirect,
            insecure,
            domain_resolve: domain_resolve.as_ref().map(|d| Arc::new(d.clone())),
        }
    }
    pub fn get_id(&self) -> &str {
        &self.id
    }
    pub fn is_insecure(&self) -> bool {
        self.insecure
    }
    pub fn get_url(&self) -> &Url {
        &self.url
    }
    pub fn get_headers(&self) -> Option<Arc<HeaderMap<HeaderValue>>> {
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
    pub fn get_timeout(&self) -> Duration {
        self.timeout
    }
    pub fn get_connect_timeout(&self) -> Duration {
        self.connect_timeout
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
    pub fn get_domain_resolve(&self) -> Option<Arc<HashMap<String, Vec<SocketAddr>>>> {
        self.domain_resolve.clone()
    }
}
