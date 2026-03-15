use std::{collections::HashMap, net::SocketAddr, sync::Arc, time::Duration};
use reqwest::{Method, header::{HeaderMap, HeaderValue}};
use url::Url;
use crate::cores::{downloader::{download_manager::DownloadManager, item::{ArcFunctionClientBuilderFallback, ArcFunctionExecuteDownload, ArcFunctionExecuteProgress, Item}, settings::{DEFAULT_CONNECT_TIMEOUT, DEFAULT_FOLLOW_REDIRECT, DEFAULT_MAX_REDIRECTS, DEFAULT_MAX_RETRIES, DEFAULT_TIMEOUT, MAX_CONNECT_TIMEOUT, MAX_MAX_RETRIES, MAX_REDIRECTS, MAX_TIMEOUT, MIN_CONNECT_TIMEOUT, MIN_MAX_RETRIES, MIN_TIMEOUT}}, idna::domain::Domain, net::dns::{DnsChoice}, system::error::{Error, ResultError}};


#[derive(Debug)]
pub struct Builder {
    download_manager: Arc<DownloadManager>,
    url: Url,
    dns: Option<Arc<DnsChoice>>,
    headers: Option<HeaderMap<HeaderValue>>,
    method: Method,
    filename: Option<String>,
    max_retries: usize,
    max_redirects: usize,
    follow_redirect: bool,
    insecure: bool,
    timeouts: Duration,
    connect_timeout: Duration,
    domain_resolve: Option<HashMap<String, Vec<SocketAddr>>>,
    on_start: Option<ArcFunctionExecuteProgress>,
    on_progress: Option<ArcFunctionExecuteProgress>,
    on_download_progress: Option<ArcFunctionExecuteDownload>,
    on_complete: Option<ArcFunctionExecuteProgress>,
    on_fail: Option<ArcFunctionExecuteProgress>,
    on_cancel: Option<ArcFunctionExecuteProgress>,
    on_finish: Option<ArcFunctionExecuteProgress>,
    builder_fallback: Option<ArcFunctionClientBuilderFallback>,
}

impl Builder {
    fn clean_domain_name(domain: &str) -> String {
        let end = domain.find(|c| c == '/' || c == '?' || c == '#' || c == ':').unwrap_or(domain.len());
        domain[..end].to_string()
    }

    pub fn new<U: Into<String>>(
        download_manager: Arc<DownloadManager>,
        url: U,
    ) -> ResultError<Self> {
        let mut url_str = url.into();
        if url_str.trim().is_empty() {
            return Err(Error::invalid_url("URL cannot be empty"));
        }
        let lower_url = url_str.to_lowercase();
        let default_protocol = "https://"; // Default to HTTPS if no protocol is provided
        let supported_protocols = vec!["http://", "https://", "ftp://", "file://"];
        if lower_url.starts_with("://") {
            // replace :// to default_protocol
            url_str = format!("{}{}", default_protocol, &url_str[3..]);
        } else if lower_url.starts_with("//") {
            // check split by // and check if the first part is a supported protocol
            let parts: Vec<&str> = lower_url.split("//").collect();
            let domain_part = parts.get(1).unwrap_or(&"");
            if domain_part.is_empty() {
                return Err(Error::invalid_url("URL cannot be empty after protocol"));
            }
            Domain::parse_only(domain_part)?; // Validate domain part
            url_str = format!("{}{}", default_protocol, &url_str[2..]);
        } else if !supported_protocols.iter().any(|p| lower_url.starts_with(p)) {
            let first_char = lower_url.chars().next().unwrap_or(' ');
            if !first_char.is_alphanumeric() {
                return Err(Error::invalid_url("URL must start with a valid protocol or domain"));
            }
            if lower_url.contains("://") {
                // get porisition for ://
                let pos = lower_url.find("://").ok_or_else(|| Error::invalid_url("URL contains invalid protocol format"))?;
                let protocol_part = &lower_url[..pos + 3];
                if !supported_protocols.iter().any(|p| protocol_part.starts_with(p)) {
                    return Err(Error::invalid_url("URL contains unsupported protocol"));
                }
                let domain_name = Self::clean_domain_name(&lower_url[pos + 3..]);
                if domain_name.is_empty() {
                    return Err(Error::invalid_url("URL cannot be empty after protocol"));
                }
                Domain::parse_only(domain_name)?; // Validate domain part
            } else {
                let mut domain_name = Self::clean_domain_name(&lower_url);
                if domain_name.is_empty() {
                    return Err(Error::invalid_url("URL cannot be empty after protocol"));
                }
                Domain::parse_only(domain_name)?; // Validate domain part
                url_str = format!("{}{}", default_protocol, url_str);
            }
        }
        let uri = Url::parse(&url_str).map_err(Error::parse_error)?;
        Ok(Self {
            url: uri,
            headers: None,
            method: Method::GET,
            download_manager: download_manager.clone(),
            max_retries: DEFAULT_MAX_RETRIES,
            max_redirects: DEFAULT_MAX_REDIRECTS,
            follow_redirect: DEFAULT_FOLLOW_REDIRECT,
            timeouts: DEFAULT_TIMEOUT,
            connect_timeout: DEFAULT_CONNECT_TIMEOUT,
            insecure: false,
            dns: download_manager.get_default_dns(),
            domain_resolve: None,
            on_start: None,
            on_progress: None,
            on_download_progress: None,
            on_complete: None,
            on_fail: None,
            on_cancel: None,
            on_finish: None,
            builder_fallback: None,
            filename: None,
        })
    }

    pub fn set_insecure(mut self, insecure: bool) -> Self {
        self.insecure = insecure;
        self
    }
    pub fn set_dns<D: Into<Arc<DnsChoice>>>(mut self, dns: Option<D>) -> Self {
        self.dns = dns.map(|e|e.into());
        self
    }
    pub fn set_filename<T: Into<String>>(mut self, filename: Option<T>) -> Self {
        self.filename = filename.map(|f| f.into());
        self
    }
    pub fn set_builder_fallback(mut self, fallback: Option<ArcFunctionClientBuilderFallback>) -> Self {
        self.builder_fallback = fallback;
        self
    }
    pub fn on_start(mut self, callback: ArcFunctionExecuteProgress) -> Self {
        self.on_start = Some(callback);
        self
    }
    pub fn on_progress(mut self, callback: ArcFunctionExecuteProgress) -> Self {
        self.on_progress = Some(callback);
        self
    }
    pub fn on_download_progress(mut self, callback: ArcFunctionExecuteDownload) -> Self {
        self.on_download_progress = Some(callback);
        self
    }
    pub fn on_complete(mut self, callback: ArcFunctionExecuteProgress) -> Self {
        self.on_complete = Some(callback);
        self
    }
    pub fn on_fail(mut self, callback: ArcFunctionExecuteProgress) -> Self {
        self.on_fail = Some(callback);
        self
    }
    pub fn on_cancel(mut self, callback: ArcFunctionExecuteProgress) -> Self {
        self.on_cancel = Some(callback);
        self
    }
    pub fn on_finish(mut self, callback: ArcFunctionExecuteProgress) -> Self {
        self.on_finish = Some(callback);
        self
    }
    pub fn set_headers(mut self, headers: Option<HeaderMap<HeaderValue>>) -> Self {
        self.headers = headers;
        self
    }
    pub fn set_method<M: Into<Method>>(mut self, method: M) -> Self {
        self.method = method.into();
        self
    }
    pub fn set_max_retries(mut self, max_retries: usize) -> Self {
        self.max_retries = max_retries.clamp(MIN_MAX_RETRIES, MAX_MAX_RETRIES);
        self
    }
    pub fn set_max_redirects(mut self, max_redirects: usize) -> Self {
        self.max_redirects = max_redirects.clamp(0, MAX_REDIRECTS);
        self
    }
    pub fn set_follow_redirect(mut self, follow_redirect: bool) -> Self {
        self.follow_redirect = follow_redirect;
        self
    }
    pub fn set_timeout(mut self, timeouts: Duration) -> Self {
        self.timeouts = timeouts.clamp(MIN_TIMEOUT, MAX_TIMEOUT);
        self
    }
    pub fn set_connect_timeout(mut self, connect_timeout: Duration) -> Self {
        self.connect_timeout = connect_timeout.clamp(MIN_CONNECT_TIMEOUT, MAX_CONNECT_TIMEOUT);
        self
    }
    pub fn set_domain_resolve(mut self, domain_resolve: Option<HashMap<String, Vec<SocketAddr>>>) -> Self {
        self.domain_resolve = domain_resolve;
        self
    }
    pub fn get_download_manager(&self) -> Arc<DownloadManager> {
        self.download_manager.clone()
    }
    pub fn get_url(&self) -> &Url {
        &self.url
    }
    pub fn get_dns(&self) -> Option<Arc<DnsChoice>> {
        self.dns.clone()
    }
    pub fn get_headers(&self) -> Option<HeaderMap<HeaderValue>> {
        self.headers.clone()
    }
    pub fn get_method(&self) -> &Method {
        &self.method
    }
    pub fn get_filename(&self) -> Option<String> {
        self.filename.clone()
    }
    pub fn get_max_retries(&self) -> usize {
        self.max_retries
    }
    pub fn get_max_redirects(&self) -> usize {
        self.max_redirects
    }
    pub fn get_follow_redirect(&self) -> bool {
        self.follow_redirect
    }
    pub fn get_timeout(&self) -> Duration {
        self.timeouts
    }
    pub fn get_connect_timeout(&self) -> Duration {
        self.connect_timeout
    }
    pub fn is_insecure(&self) -> bool {
        self.insecure
    }
    pub fn get_domain_resolve(&self) -> Option<HashMap<String, Vec<SocketAddr>>> {
        self.domain_resolve.clone()
    }
    pub fn get_on_start(&self) -> Option<ArcFunctionExecuteProgress> {
        self.on_start.clone()
    }
    pub fn get_on_progress(&self) -> Option<ArcFunctionExecuteProgress> {
        self.on_progress.clone()
    }
    pub fn get_on_download_progress(&self) -> Option<ArcFunctionExecuteDownload> {
        self.on_download_progress.clone()
    }
    pub fn get_on_complete(&self) -> Option<ArcFunctionExecuteProgress> {
        self.on_complete.clone()
    }
    pub fn get_on_fail(&self) -> Option<ArcFunctionExecuteProgress> {
        self.on_fail.clone()
    }
    pub fn get_on_cancel(&self) -> Option<ArcFunctionExecuteProgress> {
        self.on_cancel.clone()
    }
    pub fn get_on_finish(&self) -> Option<ArcFunctionExecuteProgress> {
        self.on_finish.clone()
    }
    pub fn get_builder_fallback(&self) -> Option<ArcFunctionClientBuilderFallback> {
        self.builder_fallback.clone()
    }
    pub fn build(&self) -> Item {
        Item::new(
            self.get_download_manager().clone(),
            self.get_dns().clone(),
            self.get_url().clone(),
            self.get_method().clone(),
            self.get_headers(),
            self.get_filename(),
            self.get_max_retries(),
            self.get_max_redirects(),
            self.get_follow_redirect(),
            self.get_timeout(),
            self.get_connect_timeout(),
            self.is_insecure(),
            self.get_domain_resolve(),
            self.get_on_start(),
            self.get_on_progress(),
            self.get_on_download_progress(),
            self.get_on_complete(),
            self.get_on_fail(),
            self.get_on_cancel(),
            self.get_on_finish(),
            self.get_builder_fallback(),
        )
    }
}

impl From<Item> for Builder {
    fn from(item: Item) -> Self {
        Self {
            download_manager: item.download_manager.clone(),
            url: item.metadata.url.clone(),
            dns: item.dns.clone(),
            headers: item.metadata.headers.clone().map(|e|e.as_ref().clone()),
            method: item.metadata.method.clone(),
            filename: item.metadata.filename.clone(),
            max_retries: item.metadata.max_retries,
            max_redirects: item.metadata.max_redirects,
            follow_redirect: item.metadata.follow_redirect,
            timeouts: item.metadata.timeout,
            connect_timeout: item.metadata.connect_timeout,
            insecure: item.metadata.insecure,
            domain_resolve: item.metadata.domain_resolve.clone().map(|e|e.as_ref().clone()),
            on_start: item.on_start.clone(),
            on_progress: item.on_progress.clone(),
            on_download_progress: item.on_download_progress.clone(),
            on_complete: item.on_complete.clone(),
            on_fail: item.on_fail.clone(),
            on_cancel: item.on_cancel.clone(),
            on_finish: item.on_finish.clone(),
            builder_fallback: item.builder_fallback.clone(),
        }
    }
}
