use std::{sync::Arc, time::Duration};

use crate::cores::net::dns::Dns;

pub const DEFAULT_CAPACITY: usize = 128;
pub const MIN_CAPACITY: usize = 5;
pub const MAX_CAPACITY: usize = 1024;
pub const MAX_REDIRECTS: usize = 10;
pub const DEFAULT_MAX_REDIRECTS: usize = 3;
pub const DEFAULT_MAX_RETRIES: usize = 0;
pub const MAX_MAX_RETRIES: usize = 10;
pub const MIN_MAX_RETRIES: usize = 0;
pub const DEFAULT_TIMEOUT: Duration = Duration::from_secs(30);
pub const MAX_TIMEOUT: Duration = Duration::from_secs(3600); // 1 hour
pub const MIN_TIMEOUT: Duration = Duration::from_secs(5);
pub const DEFAULT_FOLLOW_REDIRECT: bool = true;

#[derive(Debug, Clone)]
pub struct Settings {
    pub(crate) default_max_retries: usize,
    pub(crate) default_max_redirects: usize,
    pub(crate) default_follow_redirect: bool,
    pub(crate) default_timeouts: Duration,
    pub(crate) default_dns: Option<Arc<Dns>>,
}

impl Settings {
    pub(crate) fn new(
        max_retries: usize,
        max_redirects: usize,
        follow_redirect: bool,
        timeouts: Duration,
        dns: Option<Arc<Dns>>
    ) -> Self {
        Self {
            default_max_retries: max_retries.clamp(MIN_MAX_RETRIES, MAX_MAX_RETRIES),
            default_max_redirects: max_redirects.clamp(0, MAX_REDIRECTS),
            default_follow_redirect: follow_redirect,
            default_timeouts: timeouts.clamp(MIN_TIMEOUT, MAX_TIMEOUT),
            default_dns: dns.map(|d| d.clone()),
        }
    }
    pub(crate) fn set_default_max_retries(&mut self, retries: usize) {
        self.default_max_retries = retries.clamp(MIN_MAX_RETRIES, MAX_MAX_RETRIES);
    }
    pub(crate) fn set_default_max_redirects(&mut self, redirects: usize) {
        self.default_max_redirects = redirects.clamp(0, MAX_REDIRECTS);
    }
    pub(crate) fn set_default_follow_redirect(&mut self, follow: bool) {
        self.default_follow_redirect = follow;
    }
    pub(crate) fn set_default_timeouts(&mut self, timeouts: Duration) {
        self.default_timeouts = timeouts.clamp(MIN_TIMEOUT, MAX_TIMEOUT);
    }
    pub(crate) fn set_default_dns(&mut self, dns: Option<Arc<Dns>>) {
        self.default_dns = dns;
    }
}

impl Default for Settings {
    fn default() -> Self {
        Self::new(
            DEFAULT_MAX_RETRIES,
            MAX_REDIRECTS,
            DEFAULT_FOLLOW_REDIRECT,
            DEFAULT_TIMEOUT,
            None,
        )
    }
}
