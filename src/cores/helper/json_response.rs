use crate::cores::net::http_status::HttpStatus;
use crate::cores::system::error::{Error, ErrorType, StdError};
use serde::Serialize;
use serde_json::error::Category;

#[derive(Debug, Clone, Default)]
pub struct JsonResponse {
    pub debug: bool,
}

#[derive(Debug, Clone, Serialize)]
pub struct JsonError {
    pub error_code: HttpStatus,
    pub message: String,
    pub trace: Option<String>,
}

impl JsonError {
    pub fn serialize(&self, debug: bool) -> serde_json::Result<String> {
        if debug {
            return serde_json::to_string_pretty(self);
        }
        #[derive(Serialize)]
        struct Shadow {
            code: u16,
            error: String,
        }
        serde_json::to_string(&Shadow {
            code: self.error_code.to_u16(),
            error: self.message.clone(),
        })
    }
}

#[derive(Debug, Clone, Serialize)]
pub struct JsonSuccess<T: 'static + Serialize + ?Sized> {
    pub data: T,
}

impl<T: 'static + Serialize> From<T> for JsonSuccess<T>
where
    JsonSuccess<T>: Sized,
{
    fn from(value: T) -> Self {
        Self { data: value }
    }
}

impl<T: 'static + Serialize + ?Sized> JsonSuccess<T> {
    pub fn serialize(&self, debug: bool) -> serde_json::Result<String> {
        if debug {
            return serde_json::to_string_pretty(self);
        }
        serde_json::to_string(&self)
    }
}

impl JsonResponse {
    pub fn new(debug: bool) -> Self {
        Self { debug }
    }
    pub fn set_debug(&mut self, debug: bool) {
        self.debug = debug;
    }
    pub fn success<T: 'static + Serialize + ?Sized>(&self, data: Box<T>) -> JsonSuccess<Box<T>> {
        JsonSuccess::from(data)
    }
    pub fn error<M: AsRef<str>>(
        &self,
        message: M,
        error_code: Option<HttpStatus>,
        trace: Option<String>,
    ) -> JsonError {
        let error_code = if let Some(code) = error_code {
            code
        } else {
            HttpStatus::InternalServerError
        };
        JsonError {
            error_code,
            message: message.as_ref().to_string(),
            trace,
        }
    }
    pub fn with_error(&self, error: StdError) -> JsonError {
        let err = Error::from_error(error);
        let msg = format!("{:?}", err.message);
        self.error(
            err.message,
            Some(err.error_type.to_http_status()),
            Some(msg),
        )
    }
}

impl From<serde_json::Error> for JsonError {
    fn from(e: serde_json::Error) -> Self {
        let error_code = match e.classify() {
            Category::Io => ErrorType::Interrupted.to_http_status(),
            Category::Syntax => ErrorType::Parse.to_http_status(),
            Category::Data => ErrorType::InvalidData.to_http_status(),
            Category::Eof => ErrorType::UnexpectedEof.to_http_status(),
        };
        Self {
            error_code,
            message: e.to_string(),
            trace: Some(format!("{:?}", e)),
        }
    }
}

impl From<Error> for JsonError {
    fn from(value: Error) -> Self {
        let trace = Some(format!("{:?}", value));
        Self {
            error_code: value.error_type.to_http_status(),
            message: value.message.clone(),
            trace,
        }
    }
}

impl From<std::io::Error> for JsonError {
    fn from(e: std::io::Error) -> Self {
        Self::from(Error::from_io_error(e))
    }
}
