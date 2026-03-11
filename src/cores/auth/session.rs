use crate::cores::auth::session_tokenizer::{SessionPayload, SessionTokenizer};
use crate::cores::system::error::{Error, ResultError};
use actix_web::cookie::Cookie;
use actix_web::dev::ServiceRequest;
use std::sync::{Arc, OnceLock};
use std::time::Duration;

pub const DEFAULT_RENEW_DURATION: Duration = Duration::from_mins(30);
pub const MIN_RENEW_DURATION: Duration = Duration::from_mins(1);

// todo: Completing
#[derive(Clone, Debug)]
pub struct SessionManager {
    tokenizer: Arc<SessionTokenizer>,
    session_name: String,
}

#[derive(Clone, Debug)]
pub struct Session {
    cookie_name: String,
    cookie: Option<Cookie<'static>>,
    tokenizer: Arc<SessionTokenizer>,
    payload: OnceLock<Arc<ResultError<SessionPayload>>>,
    renew_duration: Duration,
}

impl Session {
    pub fn new(
        cookie_name: &str,
        cookie: Option<Cookie<'static>>,
        tokenizer: Arc<SessionTokenizer>,
    ) -> Self {
        Self {
            cookie_name: cookie_name.to_string(),
            cookie,
            tokenizer,
            payload: OnceLock::new(),
            renew_duration: DEFAULT_RENEW_DURATION,
        }
    }
    pub fn set_renew_duration(&mut self, renew_duration: Duration) -> &Duration {
        self.renew_duration = if renew_duration < MIN_RENEW_DURATION {
            MIN_RENEW_DURATION
        } else {
            renew_duration
        };
        &self.renew_duration
    }
    pub fn get_tokenizer(&self) -> Arc<SessionTokenizer> {
        self.tokenizer.clone()
    }
    pub fn get_cookie_name(&self) -> &str {
        &self.cookie_name
    }
    pub fn get_cookie(&self) -> Option<Cookie<'static>> {
        self.cookie.clone()
    }
    pub fn is_valid_payload(&self) -> bool {
        self.get_payload().is_ok()
    }
    pub fn get_payload(&self) -> &ResultError<SessionPayload> {
        &self.payload.get_or_init(|| {
            let e = self
                .get_cookie()
                .map(|e| self.tokenizer.parse(e.value()))
                .unwrap_or_else(|| Err(Error::not_found("Cookie is empty")));
            Arc::new(e)
        })
    }
    pub fn regenerate_payload<I: Into<u64>>(&self, user: I) -> ResultError<SessionPayload> {
        self.get_tokenizer().generate(user.into())
    }
}

impl SessionManager {
    pub fn new(session_name: &str, tokenizer: Arc<SessionTokenizer>) -> Self {
        Self {
            tokenizer,
            session_name: session_name.to_string(),
        }
    }

    pub fn session_name(&self) -> &str {
        &self.session_name
    }

    pub fn session_from_cookie(&self, cookie: Cookie<'static>) -> Session {
        Session::new(self.session_name(), Some(cookie), self.tokenizer.clone())
    }

    pub fn session_from_request(&self, request: &ServiceRequest) -> Session {
        Session::new(
            self.session_name(),
            request.cookie(self.session_name()),
            self.tokenizer.clone(),
        )
    }
}
