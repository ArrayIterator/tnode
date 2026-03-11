use std::error::Error;
use std::fmt::Display;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum CacheErrorKind {
    InvalidKey,
    InvalidValue,
    NotFound,
    Expired,
    Regenerating,
    InternalError,
    Unknown,
    IoError,
    ConversionError,
}

impl Display for CacheErrorKind {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{:?}", self)
    }
}

#[derive(Debug)]
pub struct CacheError {
    pub message: String,
    pub kind: CacheErrorKind,
}

impl Display for CacheError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "[{}] {}", self.kind(), self.message())
    }
}

impl Error for CacheError {}

impl CacheError {
    pub fn kind(&self) -> CacheErrorKind {
        self.kind
    }
    pub fn message(&self) -> &str {
        &self.message
    }
    pub fn new<T: AsRef<str>>(kind: CacheErrorKind, message: T) -> Self {
        Self {
            kind,
            message: message.as_ref().to_string(),
        }
    }
    pub fn invalid_key<T: AsRef<str>>(msg: T) -> Self {
        Self::new(CacheErrorKind::InvalidKey, msg)
    }
    pub fn invalid_value<T: AsRef<str>>(msg: T) -> Self {
        Self::new(CacheErrorKind::InvalidValue, msg)
    }
    pub fn not_found<T: AsRef<str>>(msg: T) -> Self {
        Self::new(CacheErrorKind::NotFound, msg)
    }
    pub fn expired<T: AsRef<str>>(msg: T) -> Self {
        Self::new(CacheErrorKind::Expired, msg)
    }
    pub fn regenerating<T: AsRef<str>>(msg: T) -> Self {
        Self::new(CacheErrorKind::Regenerating, msg)
    }
    pub fn internal_error<T: AsRef<str>>(msg: T) -> Self {
        Self::new(CacheErrorKind::InternalError, msg)
    }
    pub fn unknown<T: AsRef<str>>(msg: T) -> Self {
        Self::new(CacheErrorKind::Unknown, msg)
    }
    pub fn io_error<T: AsRef<str>>(msg: T) -> Self {
        Self::new(CacheErrorKind::IoError, msg)
    }
    pub fn conversion_error<T: AsRef<str>>(msg: T) -> Self {
        Self::new(CacheErrorKind::ConversionError, msg)
    }
}

pub type ResultCacheError<T> = Result<T, CacheError>;
