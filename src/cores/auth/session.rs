use crate::cores::auth::session_flash::{Flash, FlashItem, FlashItems, FlashManager, FlashMessage, FlashMetadata, Kind};
use crate::cores::auth::session_tokenizer::{SessionPayload, SessionTokenizer};
use crate::cores::helper::year_month_day::{DURATION_HOUR, DURATION_WEEK, DURATION_YEAR, HOUR_IN_SECONDS, MINUTE_IN_SECONDS, YEAR_IN_SECONDS};
use crate::cores::system::error::{Error, ResultError};
use actix_web::HttpMessage;
use actix_web::cookie::{Cookie, Expiration, time};
use actix_web::dev::ServiceRequest;
use std::sync::{Arc};
use std::time::Duration;

pub const DEFAULT_RENEW_DURATION: Duration = DURATION_HOUR;
pub const DEFAULT_COOKIE_DURATION: Duration = DURATION_WEEK;
pub const MIN_RENEW_DURATION: Duration = Duration::from_secs(5 * MINUTE_IN_SECONDS);
pub const MAX_RENEW_DURATION: Duration = Duration::from_secs(YEAR_IN_SECONDS - HOUR_IN_SECONDS); // add sampling
pub const MIN_COOKIE_DURATION: Duration = Duration::from_secs(15 * MINUTE_IN_SECONDS);
pub const MAX_COOKIE_DURATION: Duration = DURATION_YEAR;
pub const ADDITIONAL_COOKIE_DURATION: Duration = Duration::from_secs(15 * MINUTE_IN_SECONDS); // add 15 minutes for sampling

// todo: Completing
#[derive(Clone, Debug)]
pub struct SessionManager {
    tokenizer: Arc<SessionTokenizer>,
    session_name: String,
    flash_manager: Arc<FlashManager>,
}

#[derive(Clone, Debug)]
pub struct Session {
    cookie_name: String,
    cookie: Option<Arc<Cookie<'static>>>,
    payload: Arc<SessionPayload>,
    payload_init_error: Option<Error>,
    tokenizer: Arc<SessionTokenizer>,
    renew_duration: Duration,
    default_duration: Option<Duration>,
    original_payload: Option<Arc<SessionPayload>>,
    original_cookie: Option<Arc<Cookie<'static>>>,
    flash_manager: Arc<FlashManager>,
    current_flash: Option<Arc<Flash>>
}

impl Session {
    pub fn new(
        request: &ServiceRequest,
        flash_manager: Arc<FlashManager>,
        cookie_name: &str,
        cookie: Option<Cookie<'static>>,
        tokenizer: Arc<SessionTokenizer>,
        renew_duration: Option<Duration>,
        default_duration: Option<Duration>,
    ) -> Self {
        let renew_duration = renew_duration.unwrap_or(DEFAULT_RENEW_DURATION).clamp(MIN_RENEW_DURATION, MAX_RENEW_DURATION);
        let default_duration = default_duration.map(|e|e.clamp(MIN_COOKIE_DURATION, MAX_COOKIE_DURATION));
        let mut payload_init_error = None;
        let mut original_payload = None;
        let duration_created = default_duration.unwrap_or(DEFAULT_COOKIE_DURATION);
        let payload = match &cookie {
            Some(cookie) => {
                match tokenizer.parse(cookie.value()) {
                    Ok(payload) => {
                        let payload = Arc::new(payload.clone());
                        original_payload.replace(payload.clone());
                        payload
                    },
                    Err(e) => {
                        payload_init_error.replace(e);
                        Arc::new(tokenizer.generate_unchecked(0, duration_created))
                    }
                }
            },
            None => Arc::new(tokenizer.generate_unchecked(0, duration_created)),
        };
        let cookie = cookie.map(|c| Arc::new(c));
        let original_cookie = cookie.clone();
        let session_id = payload.token();
        // claim previous
        let mut current_flash = flash_manager.remove(session_id);
        if current_flash.is_none() {
            // reuse
            current_flash = request.extensions().get::<Arc<Flash>>().map(|e|e.clone());
        } else if let Some(flash) = &current_flash {
            // set up
            request.extensions_mut().insert(flash.clone());
        }
        Self {
            flash_manager,
            current_flash,
            cookie_name: cookie_name.to_string(),
            payload_init_error,
            payload,
            original_payload,
            cookie,
            original_cookie,
            tokenizer,
            renew_duration,
            default_duration
        }
    }
    pub fn set_renew_duration(&mut self, renew_duration: Duration) -> &Duration {
        self.renew_duration = renew_duration.clamp(MIN_RENEW_DURATION, MAX_RENEW_DURATION);
        &self.renew_duration
    }
    pub fn is_need_renew(&self) -> bool {
        self.payload.is_need_renew(self.renew_duration)
    }
    pub fn get_tokenizer(&self) -> Arc<SessionTokenizer> {
        self.tokenizer.clone()
    }
    pub fn get_cookie_name(&self) -> &str {
        &self.cookie_name
    }
    pub fn get_cookie(&self) -> Option<Arc<Cookie<'static>>> {
        self.cookie.clone()
    }
    pub fn get_original_cookie(&self) -> Option<Arc<Cookie<'static>>> {
        self.original_cookie.clone()
    }
    pub fn get_payload(&self) -> Arc<SessionPayload> {
        self.payload.clone()
    }
    pub fn get_original_payload(&self) -> Option<Arc<SessionPayload>> {
        self.original_payload.clone()
    }
    pub fn replace_payload_with_user<I: Into<u64>>(
        &mut self,
        user: I,
        duration: Option<Duration>
    ) -> ResultError<(Arc<Cookie<'static>>, Arc<SessionPayload>, Arc<SessionPayload>)> {
        let has_duration = duration.is_some();
        let expires = match duration {
            Some(d) => {
                let d = d.clamp(MIN_COOKIE_DURATION, MAX_COOKIE_DURATION);
                Expiration::DateTime(
                    time::OffsetDateTime::now_utc() + d
                )
            },
            None => Expiration::Session,
        };
        let mut duration_sampling = duration.unwrap_or(DEFAULT_COOKIE_DURATION);
        if duration_sampling < self.renew_duration {
            duration_sampling = self.renew_duration + ADDITIONAL_COOKIE_DURATION;
        }
        match self.get_tokenizer().generate(user.into(), duration_sampling) {
            Ok(payload) => {
                let token = payload.token.clone();
                let payload = Arc::new(payload);
                let previous = self.payload.clone();
                let cookie_name = self.cookie_name.clone();
                let cookie = match self.get_cookie() {
                    Some(c) => {
                        let mut c = c.as_ref().clone();
                        c.set_value(token);
                        if has_duration {
                            c.set_expires(expires);
                        }
                        Arc::new(c)
                    },
                    None => {
                        let mut c = Cookie::build(cookie_name, token)
                            // 1 year
                            .expires(expires)
                            .path("/")
                            .http_only(true) // to make sure javascript can not read
                            .finish();
                        Arc::new(c)
                    }
                };
                self.cookie = Some(cookie.clone());
                self.payload = payload.clone();
                Ok((cookie, payload, previous))
            }
            Err(e) => Err(e)
        }
    }

    pub fn get_flash_items<K: Into<Kind>>(&self, kind: K) -> Option<Arc<FlashItems>> {
        match &self.current_flash {
            Some(flash) => flash.get(kind),
            None => None,
        }
    }

    pub fn get_flash_item<K: Into<Kind>, Id: Into<String>>(&self, kind: K, id: Id) -> Option<Arc<FlashItem>> {
        match self.get_flash_items(kind) {
            Some(flash) => {
                flash.get(id)
            },
            None => None,
        }
    }

    pub fn get_flash_success_item<Id: Into<String>>(&self, id: Id) -> Option<Arc<FlashItem>> {
        self.get_flash_item(Kind::Success, id)
    }
    pub fn get_flash_error_item<Id: Into<String>>(&self, id: Id) -> Option<Arc<FlashItem>> {
        self.get_flash_item(Kind::Error, id)
    }
    pub fn get_flash_warning_item<Id: Into<String>>(&self, id: Id) -> Option<Arc<FlashItem>> {
        self.get_flash_item(Kind::Warning, id)
    }
    pub fn get_flash_info_item<Id: Into<String>>(&self, id: Id) -> Option<Arc<FlashItem>> {
        self.get_flash_item(Kind::Info, id)
    }
    pub fn flash<Id: Into<String>, K: Into<Kind>, M: Into<FlashMessage>>(&self, id: Id, kind: K, message: M, metadata: Option<FlashMetadata>) -> Arc<FlashItem> {
        self.flash_manager.flash(self.payload.token(), id, kind, message, metadata)
    }
    pub fn flash_message<Id: Into<String>, K: Into<Kind>, M: Into<FlashMessage>>(&self, id: Id, kind: K, message: M) -> Arc<FlashItem> {
        self.flash_manager.flash_message(self.payload.token(), id, kind, message)
    }
    pub fn flash_success<Id: Into<String>,M: Into<FlashMessage>>(&self, id: Id, message: M) -> Arc<FlashItem> {
        self.flash_manager.flash_success(self.payload.token(), id, message)
    }
    pub fn flash_error<Id: Into<String>,M: Into<FlashMessage>>(&self, id: Id, message: M) -> Arc<FlashItem> {
        self.flash_manager.flash_error(self.payload.token(), id, message)
    }
    pub fn flash_warning<Id: Into<String>,M: Into<FlashMessage>>(&self, id: Id, message: M) -> Arc<FlashItem> {
        self.flash_manager.flash_warning(self.payload.token(), id, message)
    }
    pub fn flash_info<Id: Into<String>,M: Into<FlashMessage>>(&self, id: Id, message: M) -> Arc<FlashItem> {
        self.flash_manager.flash_info(self.payload.token(), id, message)
    }
    pub fn keep_flash(&self) {
        if let Some(flash) = &self.current_flash {
            self.flash_manager.append_arc(flash.clone());
        }
    }
}

impl SessionManager {
    pub fn new(session_name: &str, tokenizer: Arc<SessionTokenizer>) -> Self {
        Self {
            tokenizer,
            session_name: session_name.to_string(),
            flash_manager: Arc::new(FlashManager::use_default(session_name))
        }
    }

    pub fn session_name(&self) -> &str {
        &self.session_name
    }

    pub fn session_from_request(&self, request: &ServiceRequest) -> Session {
        Session::new(
            request,
            self.flash_manager.clone(),
            self.session_name(),
            request.cookie(self.session_name()),
            self.tokenizer.clone(),
            None,
            None
        )
    }
}
