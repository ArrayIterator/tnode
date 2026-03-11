use crate::cores::generator::uuid::{Uuid, UuidCrateVersion};
use crate::cores::system::error::Error;
use actix_web::dev::{ConnectionInfo, ServiceRequest, ServiceResponse};
use actix_web::http::header::HeaderMap;
use actix_web::http::{Method, Uri, Version};
use actix_web::HttpMessage;
use std::fmt::Display;
use std::net::SocketAddr;
use std::sync::Arc;

#[derive(Clone, Debug)]
pub enum RequestType {
    JSON,
    Multipart,
    FormUrlEncoded,
    WebSocket,
    Custom(String),
}

impl RequestType {
    pub fn as_name(&self) -> String {
        match self {
            Self::JSON => "json",
            Self::Multipart => "multipart",
            Self::FormUrlEncoded => "form_urlencoded",
            Self::WebSocket => "websocket",
            Self::Custom(name) => name.as_str(),
        }
        .to_lowercase()
        .to_string()
    }
    pub fn from_name_option(name: &str) -> Option<Self> {
        let name = name.trim().to_lowercase();
        match name.as_ref() {
            "json" => Some(Self::JSON),
            "multipart" => Some(Self::Multipart),
            "form_urlencoded" => Some(Self::FormUrlEncoded),
            "websocket" => Some(Self::WebSocket),
            e => Some(Self::Custom(name.to_string())),
        }
    }
    pub fn from_name(name: &str) -> Self {
        let name = name.trim().to_lowercase();
        match name.as_ref() {
            "json" => Self::JSON,
            "multipart" => Self::Multipart,
            "form_urlencoded" => Self::FormUrlEncoded,
            "websocket" => Self::WebSocket,
            e => Self::Custom(name.to_string()),
        }
    }
}

impl Display for RequestType {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{:?}", self)
    }
}

#[derive(Clone, Debug)]
pub struct HttpRequestEvent {
    pub uuid: UuidCrateVersion,
    pub scheme: String,
    pub real_ip: Option<String>,
    pub uri: Uri,
    pub host: Option<String>,
    pub path: String,
    pub method: Method,
    pub query: String,
    pub peer_addr: Option<SocketAddr>,
    pub connection_info: Arc<ConnectionInfo>,
    pub version: Version,
    pub headers: Arc<HeaderMap>,
    pub request_type: RequestType,
}

#[derive(Clone, Debug)]
pub struct HttpResponseInner {
    pub status: actix_web::http::StatusCode,
    pub headers: Arc<HeaderMap>,
}

#[derive(Clone, Debug)]
pub struct HttpResponseEvent {
    pub request: Arc<HttpRequestEvent>,
    pub response: Option<HttpResponseInner>,
    pub error: Option<Error>,
}

impl HttpRequestEvent {
    fn new(value: &ServiceRequest, req_id: UuidCrateVersion) -> Self {
        let uri = value.uri();
        let connection_info = value.connection_info();
        let scheme = connection_info.scheme();
        let real_ip = connection_info.realip_remote_addr().map(|e| e.to_string());
        let content_type = value
            .headers()
            .get("content-type")
            .and_then(|v| v.to_str().ok())
            .unwrap_or("");
        let is_websocket_conn = scheme == "ws" || scheme == "wss";
        let request_type = if is_websocket_conn {
            RequestType::WebSocket
        } else if content_type.starts_with("application/json") {
            RequestType::JSON
        } else if content_type.starts_with("multipart/form-data") {
            RequestType::Multipart
        } else if content_type.starts_with("application/x-www-form-urlencoded") {
            RequestType::FormUrlEncoded
        } else {
            RequestType::Custom(content_type.to_string())
        };
        Self {
            uuid: req_id,
            scheme: connection_info.scheme().to_string(),
            real_ip,
            uri: uri.clone(),
            host: match uri.host() {
                None => None,
                Some(e) => Some(e.to_string()),
            },
            request_type,
            path: value.path().to_string(),
            method: value.method().clone(),
            query: value.query_string().to_string(),
            connection_info: Arc::new(connection_info.clone()),
            version: value.version(),
            headers: Arc::new(value.headers().clone()),
            peer_addr: value.peer_addr()
        }
    }

    pub fn uuid(&self) -> &UuidCrateVersion {
        &self.uuid
    }
    pub fn get_header(&self, key: &str) -> Option<&str> {
        self.headers.get(key).and_then(|e| e.to_str().ok())
    }
    pub fn scheme(&self) -> &str {
        &self.scheme
    }
    pub fn is_http_request(&self) -> bool {
        self.is_http() || self.is_websocket()
    }
    pub fn is_https(&self) -> bool {
        self.scheme() == "https"
    }
    pub fn is_http(&self) -> bool {
        self.scheme() == "http"
    }
    pub fn is_websocket(&self) -> bool {
        self.is_websocket_upgrade_request()
    }
    pub fn is_websocket_upgrade_request(&self) -> bool {
        let has_upgrade_header = self
            .headers()
            .get(actix_web::http::header::UPGRADE)
            .and_then(|v| v.to_str().ok())
            .map(|v| v.to_lowercase() == "websocket")
            .unwrap_or(false);
        let has_connection_upgrade = self
            .headers()
            .get(actix_web::http::header::CONNECTION)
            .and_then(|v| v.to_str().ok())
            .map(|v| v.to_lowercase().contains("upgrade"))
            .unwrap_or(false);
        has_upgrade_header && has_connection_upgrade
    }
    /// Do not use this function for security purposes unless you can be sure
    /// that the client cannot spoof the Forwarded and X-Forwarded-For headers.
    /// If you are running without a proxy, then getting the peer address would be more appropriate.
    /// Use [self::connection_info](Self::connection_info) instead
    /// ```rust
    /// // example usage peer connection info
    /// let connection_info = req.connection_info();
    /// let source_ip: Option<String> = connection_info.peer_addr().map(|e| e.to_string());
    /// ```
    pub fn real_ip(&self) -> Option<&str> {
        self.real_ip.as_deref()
    }
    pub fn uri(&self) -> &Uri {
        &self.uri
    }
    pub fn host(&self) -> Option<&str> {
        self.host.as_deref()
    }
    pub fn path(&self) -> &str {
        &self.path
    }
    pub fn method(&self) -> &Method {
        &self.method
    }
    pub fn query(&self) -> &str {
        &self.query
    }
    pub fn peer_addr(&self) -> Option<SocketAddr> {
        self.peer_addr.clone()
    }
    pub fn connection_info(&self) -> &ConnectionInfo {
        &self.connection_info
    }
    pub fn version(&self) -> &Version {
        &self.version
    }
    pub fn headers(&self) -> &HeaderMap {
        &self.headers
    }
    pub fn request_type(&self) -> &RequestType {
        &self.request_type
    }
}

impl HttpResponseEvent {
    pub fn new(
        request: Arc<HttpRequestEvent>,
        response: &Result<ServiceResponse, actix_web::Error>,
    ) -> Self {
        let mut error = None;
        let mut res = None;
        let response = response.as_ref();
        match response {
            Ok(response) => {
                let status = response.status();
                let headers = Arc::new(response.headers().clone());
                res = Some(HttpResponseInner { status, headers });
            }
            Err(e) => error = Some(Error::from(e)),
        }
        Self {
            request,
            error,
            response: res,
        }
    }
}

impl From<&ServiceRequest> for HttpRequestEvent {
    fn from(req: &ServiceRequest) -> Self {
        let uuid = if let Some(version) = req.extensions().get::<UuidCrateVersion>() {
            *version
        } else {
            Uuid::v7()
        };
        Self::new(req, uuid)
    }
}

#[derive(Clone, Debug)]
pub struct HttpResultEvent {
    pub request: HttpRequestEvent,
}
