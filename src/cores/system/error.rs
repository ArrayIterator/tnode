use crate::cores::net::http_status::HttpStatus;
use std::error;
use std::fmt::{Debug, Display, Formatter};
use std::io::ErrorKind;
use std::sync::Arc;

/// ```rust
/// Represents various error types that can occur in a system or application.
///
/// # Variants
///
/// - `FileNotFound`: The specified file was not found.
/// - `AddressInUse`: The requested address is already in use.
/// - `DeadLock`: A deadlock was detected.
/// - `NotFound`: The requested item was not found.
/// - `PermissionDenied`: The operation was denied due to insufficient permissions.
/// - `ConnectionRefused`: A connection attempt was refused by the target.
/// - `ConnectionReset`: The connection was reset by a peer.
/// - `HostUnreachable`: The remote host could not be reached.
/// - `NetworkUnreachable`: The network is unreachable.
/// - `ConnectionAborted`: The connection was aborted.
/// - `NotConnected`: The operation requires an established connection, but none exists.
/// - `AddrInUse`: The address is already in use.
/// - `AddrNotAvailable`: The address is not available to bind to.
/// - `NetworkDown`: The network is down.
/// - `BrokenPipe`: A pipe operation failed due to a broken pipe.
/// - `AlreadyExists`: The specified item already exists.
/// - `WouldBlock`: The operation would block, and it is expected to be non-blocking.
/// - `NotADirectory`: The specified path is not a directory.
/// - `IsADirectory`: The specified path is a directory.
/// - `DirectoryNotEmpty`: The directory is not empty.
/// - `ReadOnlyFilesystem`: The filesystem is in a read-only state.
/// - `FilesystemLoop`: A loop was detected in the filesystem.
/// - `StaleNetworkFileHandle`: A network file handle is stale.
/// - `InvalidInput`: The input provided is invalid.
/// - `InvalidData`: The data provided is invalid.
/// - `TimedOut`: The operation timed out.
/// - `WriteZero`: An attempt to write yielded zero bytes written.
/// - `StorageFull`: The storage medium is full.
/// - `NotSeekable`: The resource does not support seeking.
/// - `QuotaExceeded`: The operation exceeded its quota.
/// - `FileTooLarge`: The file size exceeds the allowable limit.
/// - `ResourceBusy`: The resource is currently busy.
/// - `ExecutableFileBusy`: The executable file is busy and cannot be used.
/// - `CrossesDevices`: The operation involves crossing device boundaries.
/// - `TooManyLinks`: Too many hard links exist.
/// - `InvalidFilename`: The filename provided is invalid.
/// - `ArgumentListTooLong`: The argument list exceeds the maximum allowable size.
/// - `Interrupted`: The operation was interrupted.
/// - `Unsupported`: The requested operation is not supported.
/// - `ResourceUnavailable`: The resource is temporarily unavailable.
/// - `UnexpectedEof`: An unexpected end of file was encountered.
/// - `OutOfMemory`: The system ran out of memory.
/// - `InProgress`: The operation is already in progress.
/// - `Other`: An unspecified error occurred.
/// - `Uncategorized`: An error that does not fit other categories.
/// - `TlsHandshakeFailed`: A failure occurred during a TLS handshake.
/// - `AcmeChallengeFailed`: Failure occurred during an ACME challenge.
/// - `CertExpired`: The certificate has expired.
/// - `InvalidConfig`: The provided configuration is invalid.
/// - `Parse`: A parsing error occurred.
/// - `Encoding`: An encoding error occurred.
/// - `Overflow`: An overflow occurred during an operation.
/// - `InvalidRange`: The provided range is invalid.
/// - `InvalidLength`: The provided length is invalid.
/// - `BufferOverflow`: A buffer overflow occurred.
/// - `ChannelClosed`: A channel was closed unexpectedly.
/// - `AlreadyRunning`: The operation is already running.
/// - `RenderError`: An error occurred during rendering.
/// - `FileExists`: The file already exists.
/// - `ConversionFailed`: A conversion operation failed.
/// - `InvalidState`: The state is invalid for the operation.
/// - `Expired`: The state is expired
/// - `Future`: The state invalid timestamp
#[derive(Debug, Clone, Hash, PartialEq)]
pub enum ErrorType {
    FileNotFound,
    AddressInUse,
    DeadLock,
    NotFound,
    PermissionDenied,
    ConnectionRefused,
    ConnectionReset,
    HostUnreachable,
    NetworkUnreachable,
    ConnectionAborted,
    NotConnected,
    AddrInUse,
    AddrNotAvailable,
    NetworkDown,
    BrokenPipe,
    AlreadyExists,
    WouldBlock,
    NotADirectory,
    IsADirectory,
    DirectoryNotEmpty,
    ReadOnlyFilesystem,
    FilesystemLoop,
    StaleNetworkFileHandle,
    InvalidInput,
    InvalidUrl,
    InvalidData,
    TimedOut,
    WriteZero,
    StorageFull,
    NotSeekable,
    QuotaExceeded,
    FileTooLarge,
    ResourceBusy,
    ExecutableFileBusy,
    CrossesDevices,
    TooManyLinks,
    InvalidFilename,
    ArgumentListTooLong,
    Interrupted,
    Unsupported,
    ResourceUnavailable,
    UnexpectedEof,
    OutOfMemory,
    InProgress,
    Other,
    Uncategorized,
    TlsHandshakeFailed,
    AcmeChallengeFailed,
    CertExpired,
    InvalidConfig,
    Parse,
    Encoding,
    Overflow,
    InvalidRange,
    InvalidLength,
    BufferOverflow,
    ChannelClosed,
    AlreadyRunning,
    RenderError,
    FileExists,
    ConversionFailed,
    InvalidState,
    Expired,
    Future,
}

impl ErrorType {
    /// Converts an `ErrorType` enum variant into its corresponding HTTP status code.
    ///
    /// This method maps specific error types to appropriate HTTP status codes
    /// based on the nature of the error as follows:
    ///
    /// ### Mappings:
    ///
    /// - **400 Bad Request**
    ///   - Occurs for client-side errors related to invalid inputs or invalid configurations:
    ///     - `ErrorType::InvalidInput`
    ///     - `ErrorType::InvalidData`
    ///     - `ErrorType::InvalidFilename`
    ///     - `ErrorType::ArgumentListTooLong`
    ///     - `ErrorType::Parse`
    ///     - `ErrorType::Encoding`
    ///     - `ErrorType::InvalidConfig`
    ///     - `ErrorType::InvalidRange`
    ///     - `ErrorType::InvalidLength`
    ///     - `ErrorType::Overflow`
    ///     - `ErrorType::UnexpectedEof`
    ///
    /// - **403 Forbidden**
    ///   - Indicates lack of permissions or read-only conditions:
    ///     - `ErrorType::PermissionDenied`
    ///     - `ErrorType::ReadOnlyFilesystem`
    ///
    /// - **404 Not Found**
    ///   - For errors related to missing files or directories:
    ///     - `ErrorType::FileNotFound`
    ///     - `ErrorType::NotFound`
    ///     - `ErrorType::NotADirectory`
    ///
    /// - **408 Request Timeout**
    ///   - For timeouts during operations:
    ///     - `ErrorType::TimedOut`
    ///
    /// - **409 Conflict**
    ///   - For errors caused by conflicts or resource contention:
    ///     - `ErrorType::AlreadyExists`
    ///     - `ErrorType::AddrInUse`
    ///     - `ErrorType::AddressInUse`
    ///     - `ErrorType::AlreadyRunning`
    ///     - `ErrorType::FileExists`
    ///     - `ErrorType::DirectoryNotEmpty`
    ///
    /// - **413 Payload Too Large**
    ///   - For file or buffer size issues:
    ///     - `ErrorType::FileTooLarge`
    ///     - `ErrorType::BufferOverflow`
    ///
    /// - **429 Too Many Requests**
    ///   - Triggered when quotas are exceeded:
    ///     - `ErrorType::QuotaExceeded`
    ///
    /// - **500 Internal Server Error**
    ///   - General server-side or unexpected errors:
    ///     - `ErrorType::DeadLock`
    ///     - `ErrorType::OutOfMemory`
    ///     - `ErrorType::Other`
    ///     - `ErrorType::Uncategorized`
    ///     - `ErrorType::Interrupted`
    ///     - `ErrorType::WriteZero`
    ///     - `ErrorType::NotSeekable`
    ///     - `ErrorType::CrossesDevices`
    ///     - `ErrorType::TooManyLinks`
    ///     - `ErrorType::FilesystemLoop`
    ///     - `ErrorType::StaleNetworkFileHandle`
    ///     - `ErrorType::ChannelClosed`
    ///
    /// - **502 Bad Gateway**
    ///   - For upstream connectivity or network failures:
    ///     - `ErrorType::TlsHandshakeFailed`
    ///     - `ErrorType::AcmeChallengeFailed`
    ///     - `ErrorType::CertExpired`
    ///     - `ErrorType::HostUnreachable`
    ///     - `ErrorType::NetworkUnreachable`
    ///     - `ErrorType::NetworkDown`
    ///     - `ErrorType::ConnectionRefused`
    ///     - `ErrorType::ConnectionReset`
    ///     - `ErrorType::ConnectionAborted`
    ///     - `ErrorType::NotConnected`
    ///     - `ErrorType::AddrNotAvailable`
    ///     - `ErrorType::BrokenPipe`
    ///
    /// - **503 Service Unavailable**
    ///   - Denotes temporary unavailability of resources:
    ///     - `ErrorType::ResourceBusy`
    ///     - `ErrorType::ResourceUnavailable`
    ///     - `ErrorType::InProgress`
    ///     - `ErrorType::ExecutableFileBusy`
    ///     - `ErrorType::WouldBlock`
    ///
    /// - **507 Insufficient Storage**
    ///   - For storage capacity exhaustion:
    ///     - `ErrorType::StorageFull`
    ///
    /// - **501 Not Implemented**
    ///   - Fallback for unsupported operations:
    ///     - `ErrorType::Unsupported`
    ///
    /// - **500 Internal Server Error (Default Fallback)**
    ///   - Used if no other mapping exists:
    ///     - `HttpStatus::InternalServerError`
    ///
    /// ### Returns:
    /// - `HttpStatus`: The associated HTTP status code for the given `ErrorType`.
    ///
    /// ### Example:
    /// ```rust
    /// let error_type = ErrorType::InvalidInput;
    /// let status = error_type.to_http_status();
    /// assert_eq!(status, HttpStatus::BadRequest);
    /// ```
    pub fn to_http_status(&self) -> HttpStatus {
        match self {
            // 400 Bad Request
            ErrorType::InvalidInput
            | ErrorType::InvalidUrl
            | ErrorType::InvalidData
            | ErrorType::InvalidFilename
            | ErrorType::ArgumentListTooLong
            | ErrorType::Parse
            | ErrorType::Encoding
            | ErrorType::InvalidConfig
            | ErrorType::InvalidRange
            | ErrorType::InvalidLength
            | ErrorType::Overflow
            | ErrorType::UnexpectedEof => HttpStatus::BadRequest,

            // 403 Forbidden
            ErrorType::PermissionDenied | ErrorType::ReadOnlyFilesystem => HttpStatus::Forbidden,

            // 404 Not Found
            ErrorType::FileNotFound | ErrorType::NotFound | ErrorType::NotADirectory => {
                HttpStatus::NotFound
            }

            // 408 Request Timeout
            ErrorType::TimedOut => HttpStatus::RequestTimeout,

            // 409 Conflict
            ErrorType::AlreadyExists
            | ErrorType::AddrInUse
            | ErrorType::AddressInUse
            | ErrorType::AlreadyRunning
            | ErrorType::FileExists
            | ErrorType::DirectoryNotEmpty => HttpStatus::Conflict,

            // 413 Payload Too Large
            ErrorType::FileTooLarge | ErrorType::BufferOverflow => {
                HttpStatus::RequestEntityTooLarge
            }

            // 429 Too Many Requests
            ErrorType::QuotaExceeded => HttpStatus::TooManyRequests,

            // 500 Internal Server Error
            ErrorType::DeadLock
            | ErrorType::OutOfMemory
            | ErrorType::Other
            | ErrorType::Uncategorized
            | ErrorType::Interrupted
            | ErrorType::WriteZero
            | ErrorType::NotSeekable
            | ErrorType::CrossesDevices
            | ErrorType::TooManyLinks
            | ErrorType::FilesystemLoop
            | ErrorType::StaleNetworkFileHandle
            | ErrorType::ChannelClosed => HttpStatus::InternalServerError,

            // 502 Bad Gateway (Upstream Problems)
            ErrorType::TlsHandshakeFailed
            | ErrorType::AcmeChallengeFailed
            | ErrorType::CertExpired
            | ErrorType::HostUnreachable
            | ErrorType::NetworkUnreachable
            | ErrorType::NetworkDown
            | ErrorType::ConnectionRefused
            | ErrorType::ConnectionReset
            | ErrorType::ConnectionAborted
            | ErrorType::NotConnected
            | ErrorType::AddrNotAvailable
            | ErrorType::BrokenPipe => HttpStatus::BadGateway,

            // 503 Service Unavailable
            ErrorType::ResourceBusy
            | ErrorType::ResourceUnavailable
            | ErrorType::InProgress
            | ErrorType::ExecutableFileBusy
            | ErrorType::WouldBlock => HttpStatus::ServiceUnavailable,

            // 507 Insufficient Storage
            ErrorType::StorageFull => HttpStatus::InsufficientStorage,

            // Default fallback
            ErrorType::Unsupported => HttpStatus::NotImplemented,
            _ => HttpStatus::InternalServerError,
        }
    }

    /// Converts an HTTP status code into a corresponding `ErrorType`.
    ///
    /// This method maps common HTTP status codes to predefined error types
    /// in the application, providing a way to interpret status codes as
    /// structured error information.
    ///
    /// # Parameters
    /// - `status`: The HTTP status code as a `u16`.
    ///
    /// # Returns
    /// - An instance of `ErrorType` that corresponds to the provided HTTP
    ///   status code. The returned `ErrorType` represents the semantic meaning
    ///   of the HTTP status code within the context of the application.
    ///
    /// # Mappings
    /// - **4xx Client Errors:**
    ///   - `400 Bad Request` → `ErrorType::InvalidInput`
    ///   - `401 Unauthorized` → `ErrorType::PermissionDenied`
    ///   - `403 Forbidden` → `ErrorType::PermissionDenied`
    ///   - `404 Not Found` → `ErrorType::NotFound`
    ///   - `405 Method Not Allowed` → `ErrorType::Unsupported`
    ///   - `408 Request Timeout` → `ErrorType::TimedOut`
    ///   - `409 Conflict` → `ErrorType::AlreadyExists`
    ///   - `413 Payload Too Large` → `ErrorType::FileTooLarge`
    ///   - `415 Unsupported Media Type` → `ErrorType::Unsupported`
    ///   - `422 Unprocessable Entity` → `ErrorType::InvalidData`
    ///   - `429 Too Many Requests` → `ErrorType::QuotaExceeded`
    /// - **5xx Server Errors:**
    ///   - `500 Internal Server Error` → `ErrorType::Other`
    ///   - `501 Not Implemented` → `ErrorType::Unsupported`
    ///   - `502 Bad Gateway` → `ErrorType::ConnectionRefused`
    ///   - `503 Service Unavailable` → `ErrorType::ResourceUnavailable`
    ///   - `504 Gateway Timeout` → `ErrorType::TimedOut`
    ///   - `507 Insufficient Storage` → `ErrorType::StorageFull`
    /// - **Fallbacks:**
    ///   - `4xx` range (excluding specific mappings) → `ErrorType::InvalidInput`
    ///   - `5xx` range (excluding specific mappings) → `ErrorType::Uncategorized`
    ///   - Any other status → `ErrorType::Other`
    ///
    /// # Example
    /// ```
    /// use my_crate::ErrorType;
    ///
    /// let error = ErrorType::from_http_status(404);
    /// assert_eq!(error, ErrorType::NotFound);
    ///
    /// let fallback_error = ErrorType::from_http_status(451);
    /// assert_eq!(fallback_error, ErrorType::InvalidInput);
    /// ```
    ///
    /// This ensures the HTTP status is meaningfully represented as an `ErrorType`.
    pub fn from_http_status(status: u16) -> Self {
        match status {
            // 400 Bad Request
            400 => ErrorType::InvalidInput,
            // 401 Unauthorized
            401 => ErrorType::PermissionDenied,
            // 403 Forbidden
            403 => ErrorType::PermissionDenied,
            // 404 Not Found
            404 => ErrorType::NotFound,
            // 405 Method Not Allowed
            405 => ErrorType::Unsupported,
            // 408 Request Timeout
            408 => ErrorType::TimedOut,
            // 409 Conflict
            409 => ErrorType::AlreadyExists,
            // 413 Payload Too Large
            413 => ErrorType::FileTooLarge,
            // 415 Unsupported Media Type
            415 => ErrorType::Unsupported,
            // 422 Unprocessable Entity
            422 => ErrorType::InvalidData,
            // 429 Too Many Requests
            429 => ErrorType::QuotaExceeded,

            // 500 Internal Server Error
            500 => ErrorType::Other,
            // 501 Not Implemented
            501 => ErrorType::Unsupported,
            // 502 Bad Gateway
            502 => ErrorType::ConnectionRefused,
            // 503 Service Unavailable
            503 => ErrorType::ResourceUnavailable,
            // 504 Gateway Timeout
            504 => ErrorType::TimedOut,
            // 507 Insufficient Storage
            507 => ErrorType::StorageFull,

            // Fallback
            400..=499 => ErrorType::InvalidInput,
            500..=599 => ErrorType::Uncategorized,
            _ => ErrorType::Other,
        }
    }
}

pub type StdError = Box<dyn error::Error + Send + Sync + 'static>;

impl From<StdError> for Error {
    fn from(err: StdError) -> Self {
        Self::from_error(err)
    }
}
impl Display for Error {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        let mut t = "".to_string();
        if let Some(e) = &self.original_error {
            t = format!(" Stack: {}", e);
        }
        write!(f, "[{:?}] {}{}", self.error_type, self.message, t)
    }
}

impl From<std::io::Error> for Error {
    fn from(err: std::io::Error) -> Self {
        Self::from_io_error(err)
    }
}
impl From<std::string::FromUtf8Error> for Error {
    fn from(err: std::string::FromUtf8Error) -> Self {
        Self::new(ErrorType::Encoding, err)
    }
}
impl From<std::num::ParseIntError> for Error {
    fn from(err: std::num::ParseIntError) -> Self {
        Self::new(ErrorType::Parse, err)
    }
}
impl From<&actix_web::Error> for Error {
    fn from(err: &actix_web::Error) -> Self {
        let status_code = err.as_response_error().status_code().as_u16();
        let message = format!("{:?}", err);
        Self {
            error_type: ErrorType::from_http_status(status_code),
            original_error: None,
            message,
        }
    }
}

#[derive(Debug, Clone)]
pub struct Error {
    pub error_type: ErrorType,
    pub message: String,
    pub original_error: Option<Arc<StdError>>,
}

impl error::Error for Error {}

impl Error {
    pub fn new<T>(error_type: ErrorType, message: T) -> Self
    where
        T: Into<StdError>,
    {
        let mut original_error = None;
        let message = message.into();
        let str = message.to_string();
        let res = message.downcast::<Error>();
        if let Ok(e) = res {
            let cloned = *e.clone();
            return cloned;
        }
        Self {
            error_type,
            original_error,
            message: str,
        }
    }
    pub fn kind(&self) -> ErrorType {
        self.error_type.clone()
    }
    pub fn is_kind(&self, kind: ErrorType) -> bool {
        self.error_type == kind
    }
    pub fn get_original_error(&self) -> Option<&Arc<StdError>> {
        self.original_error.as_ref()
    }
    pub fn from_error<T>(err: T) -> Self
    where
        T: Into<StdError> + ToString,
    {
        let std_err: StdError = err.into();

        let err_ref: &dyn error::Error = std_err.as_ref();

        if let Some(io_err) = err_ref.downcast_ref::<std::io::Error>() {
            return Self::from_io_error(std::io::Error::new(io_err.kind(), io_err.to_string()));
        }

        if let Some(f) = err_ref.downcast_ref::<std::string::FromUtf8Error>() {
            return Error::new(ErrorType::Encoding, f.to_string());
        }

        if let Some(ac_err) = err_ref.downcast_ref::<actix_web::Error>() {
            let status_code = ac_err.as_response_error().status_code().as_u16();
            let message = format!("{:?}", ac_err);
            return Self {
                error_type: ErrorType::from_http_status(status_code),
                original_error: None,
                message,
            };
        }
        if let Some(pe) = err_ref.downcast_ref::<std::num::ParseIntError>() {
            return Self::new(ErrorType::Parse, pe.to_string());
        }

        if let Some(custom_err) = err_ref.downcast_ref::<Self>() {
            return custom_err.clone();
        }
        // 6. Aman, move std_err ke constructor
        Self::new(ErrorType::Other, std_err)
    }

    pub fn from_acme_error(err: instant_acme::Error) -> Self {
        match err {
            instant_acme::Error::Api(_) => Self::new(ErrorType::InvalidState, err),
            instant_acme::Error::Crypto => Self::new(ErrorType::InvalidState, err),
            instant_acme::Error::KeyRejected => Self::new(ErrorType::PermissionDenied, err),
            instant_acme::Error::Http(_) => Self::new(ErrorType::InvalidState, err),
            instant_acme::Error::Hyper(_) => Self::new(ErrorType::InvalidState, err),
            instant_acme::Error::InvalidUri(_) => Self::new(ErrorType::HostUnreachable, err),
            instant_acme::Error::Json(_) => Self::new(ErrorType::Parse, err),
            instant_acme::Error::Timeout(_) => Self::new(ErrorType::TimedOut, err),
            instant_acme::Error::Unsupported(_) => Self::new(ErrorType::Unsupported, err),
            instant_acme::Error::Other(_) => Self::new(ErrorType::Other, err),
            _ => Self::new(ErrorType::Other, err),
        }
    }
    pub fn from_io_error(err: std::io::Error) -> Self {
        let kind = err.kind();
        let err_type = match kind {
            ErrorKind::NotFound => ErrorType::NotFound,
            ErrorKind::PermissionDenied => ErrorType::PermissionDenied,
            ErrorKind::ConnectionRefused => ErrorType::ConnectionRefused,
            ErrorKind::ConnectionReset => ErrorType::ConnectionReset,
            ErrorKind::HostUnreachable => ErrorType::HostUnreachable,
            ErrorKind::NetworkUnreachable => ErrorType::NetworkUnreachable,
            ErrorKind::ConnectionAborted => ErrorType::ConnectionAborted,
            ErrorKind::NotConnected => ErrorType::NotConnected,
            ErrorKind::AddrInUse => ErrorType::AddrInUse,
            ErrorKind::AddrNotAvailable => ErrorType::AddrNotAvailable,
            ErrorKind::NetworkDown => ErrorType::NetworkDown,
            ErrorKind::BrokenPipe => ErrorType::BrokenPipe,
            ErrorKind::AlreadyExists => ErrorType::AlreadyExists,
            ErrorKind::WouldBlock => ErrorType::WouldBlock,
            ErrorKind::NotADirectory => ErrorType::NotADirectory,
            ErrorKind::IsADirectory => ErrorType::IsADirectory,
            ErrorKind::DirectoryNotEmpty => ErrorType::DirectoryNotEmpty,
            ErrorKind::ReadOnlyFilesystem => ErrorType::ReadOnlyFilesystem,
            ErrorKind::StaleNetworkFileHandle => ErrorType::StaleNetworkFileHandle,
            ErrorKind::InvalidInput => ErrorType::InvalidInput,
            ErrorKind::InvalidData => ErrorType::InvalidData,
            ErrorKind::TimedOut => ErrorType::TimedOut,
            ErrorKind::WriteZero => ErrorType::WriteZero,
            ErrorKind::StorageFull => ErrorType::StorageFull,
            ErrorKind::NotSeekable => ErrorType::NotSeekable,
            ErrorKind::QuotaExceeded => ErrorType::QuotaExceeded,
            ErrorKind::FileTooLarge => ErrorType::FileTooLarge,
            ErrorKind::ResourceBusy => ErrorType::ResourceBusy,
            ErrorKind::ExecutableFileBusy => ErrorType::ExecutableFileBusy,
            ErrorKind::Deadlock => ErrorType::DeadLock,
            ErrorKind::CrossesDevices => ErrorType::CrossesDevices,
            ErrorKind::TooManyLinks => ErrorType::TooManyLinks,
            ErrorKind::InvalidFilename => ErrorType::InvalidFilename,
            ErrorKind::ArgumentListTooLong => ErrorType::ArgumentListTooLong,
            ErrorKind::Interrupted => ErrorType::Interrupted,
            ErrorKind::Unsupported => ErrorType::Unsupported,
            ErrorKind::UnexpectedEof => ErrorType::UnexpectedEof,
            ErrorKind::OutOfMemory => ErrorType::OutOfMemory,
            ErrorKind::Other => ErrorType::Other,
            _ => {
                let k_str = format!("{:?}", kind);
                match k_str.as_str() {
                    "FilesystemLoop" => ErrorType::FilesystemLoop,
                    "InProgress" => ErrorType::InProgress,
                    "Uncategorized" => ErrorType::Uncategorized,
                    _ => ErrorType::Other,
                }
            }
        };
        Self::new(err_type, err)
    }

    pub fn invalid_url<T>(m: T) -> Self
    where
        T: Into<StdError>,
    {
        Self::new(ErrorType::InvalidUrl, m)
    }

    pub fn from_std_error<T>(err: StdError) -> Self
    where
        T: Into<StdError>,
    {
        Self::new(ErrorType::Other, err)
    }
    pub fn conversion_failed<T>(m: T) -> Self
    where
        T: Into<StdError>,
    {
        Self::new(ErrorType::ConversionFailed, m)
    }
    pub fn expired<T>(m: T) -> Self
    where
        T: Into<StdError>,
    {
        Self::new(ErrorType::Expired, m)
    }
    pub fn future<T>(m: T) -> Self
    where
        T: Into<StdError>,
    {
        Self::new(ErrorType::Future, m)
    }
    pub fn invalid_state<T>(m: T) -> Self
    where
        T: Into<StdError>,
    {
        Self::new(ErrorType::InvalidState, m)
    }
    pub fn interrupted<T>(m: T) -> Self
    where
        T: Into<StdError>,
    {
        Self::new(ErrorType::Interrupted, m)
    }
    pub fn parse_error<T>(m: T) -> Self
    where
        T: Into<StdError>,
    {
        Self::new(ErrorType::Parse, m)
    }
    pub fn io_error<T>(err: T) -> Self
    where
        T: Into<StdError>,
    {
        match err.into().downcast::<std::io::Error>() {
            Ok(io_err) => Self::from_io_error(*io_err),
            Err(std_err) => Self::from_io_error(std::io::Error::new(ErrorKind::Other, std_err.to_string())),
        }
    }
    pub fn tls_handshake_error<T>(err: std::io::Error, m: T) -> Self
    where
        T: Into<StdError>,
    {
        Self::new(ErrorType::TlsHandshakeFailed, m)
    }
    pub fn acme_challenge_error<T>(err: std::io::Error, m: T) -> Self
    where
        T: Into<StdError>,
    {
        Self::new(ErrorType::AcmeChallengeFailed, m)
    }
    pub fn already_exists<T>(m: T) -> Self
    where
        T: Into<StdError>,
    {
        Self::new(ErrorType::AlreadyExists, m)
    }
    pub fn cert_expired_error<T>(err: std::io::Error, m: T) -> Self
    where
        T: Into<StdError>,
    {
        Self::new(ErrorType::CertExpired, m)
    }
    pub fn invalid_config_error<T>(err: std::io::Error, m: T) -> Self
    where
        T: Into<StdError>,
    {
        Self::new(ErrorType::InvalidConfig, m)
    }
    pub fn file_not_found<T>(m: T) -> Self
    where
        T: Into<StdError>,
    {
        Self::new(ErrorType::FileNotFound, m)
    }

    pub fn address_in_use<T>(m: T) -> Self
    where
        T: Into<StdError>,
    {
        Self::new(ErrorType::AddressInUse, m)
    }

    pub fn already_running<T>(m: T) -> Self
    where
        T: Into<StdError>,
    {
        Self::new(ErrorType::AlreadyRunning, m)
    }

    pub fn in_progress<T>(m: T) -> Self
    where
        T: Into<StdError>,
    {
        Self::new(ErrorType::InProgress, m)
    }
    pub fn invalid_data<T>(m: T) -> Self
    where
        T: Into<StdError>,
    {
        Self::new(ErrorType::InvalidData, m)
    }

    pub fn dead_lock<T>(m: T) -> Self
    where
        T: Into<StdError>,
    {
        Self::new(ErrorType::DeadLock, m)
    }

    pub fn not_found<T>(m: T) -> Self
    where
        T: Into<StdError>,
    {
        Self::new(ErrorType::NotFound, m)
    }

    pub fn permission_denied<T>(m: T) -> Self
    where
        T: Into<StdError>,
    {
        Self::new(ErrorType::PermissionDenied, m)
    }

    pub fn connection_refused<T>(m: T) -> Self
    where
        T: Into<StdError>,
    {
        Self::new(ErrorType::ConnectionRefused, m)
    }

    pub fn timeout<T>(m: T) -> Self
    where
        T: Into<StdError>,
    {
        Self::new(ErrorType::TimedOut, m)
    }

    pub fn out_of_memory<T>(m: T) -> Self
    where
        T: Into<StdError>,
    {
        Self::new(ErrorType::OutOfMemory, m)
    }

    pub fn channel_closed<T>(m: T) -> Self
    where
        T: Into<StdError>,
    {
        Self::new(ErrorType::ChannelClosed, m)
    }

    pub fn address_not_available<T>(m: T) -> Self
    where
        T: Into<StdError>,
    {
        Self::new(ErrorType::AddrNotAvailable, m)
    }

    pub fn unsupported<T>(m: T) -> Self
    where
        T: Into<StdError>,
    {
        Self::new(ErrorType::Unsupported, m)
    }
    pub fn encoding<T>(m: T) -> Self
    where
        T: Into<StdError>,
    {
        Self::new(ErrorType::Encoding, m)
    }
    pub fn render_error<T>(m: T) -> Self
    where
        T: Into<StdError>,
    {
        Self::new(ErrorType::RenderError, m)
    }
    pub fn tls_failed<T>(m: T) -> Self
    where
        T: Into<StdError>,
    {
        Self::new(ErrorType::TlsHandshakeFailed, m)
    }

    pub fn acme_failed<T>(m: T) -> Self
    where
        T: Into<StdError>,
    {
        Self::new(ErrorType::AcmeChallengeFailed, m)
    }

    pub fn cert_expired<T>(m: T) -> Self
    where
        T: Into<StdError>,
    {
        Self::new(ErrorType::CertExpired, m)
    }

    pub fn invalid_config<T>(m: T) -> Self
    where
        T: Into<StdError>,
    {
        Self::new(ErrorType::InvalidConfig, m)
    }

    pub fn invalid_input<T>(m: T) -> Self
    where
        T: Into<StdError>,
    {
        Self::new(ErrorType::InvalidInput, m)
    }
    pub fn overflow<T>(m: T) -> Self
    where
        T: Into<StdError>,
    {
        Self::new(ErrorType::Overflow, m)
    }
    pub fn invalid_range<T>(m: T) -> Self
    where
        T: Into<StdError>,
    {
        Self::new(ErrorType::InvalidRange, m)
    }
    pub fn invalid_length<T>(m: T) -> Self
    where
        T: Into<StdError>,
    {
        Self::new(ErrorType::InvalidLength, m)
    }

    pub fn unexpected_eof<T>(m: T) -> Self
    where
        T: Into<StdError>,
    {
        Self::new(ErrorType::UnexpectedEof, m)
    }

    pub fn write_zero<T>(m: T) -> Self
    where
        T: Into<StdError>,
    {
        Self::new(ErrorType::WriteZero, m)
    }
    pub fn storage_full<T>(m: T) -> Self
    where
        T: Into<StdError>,
    {
        Self::new(ErrorType::StorageFull, m)
    }
    pub fn not_seekable<T>(m: T) -> Self
    where
        T: Into<StdError>,
    {
        Self::new(ErrorType::NotSeekable, m)
    }
    pub fn quota_exceeded<T>(m: T) -> Self
    where
        T: Into<StdError>,
    {
        Self::new(ErrorType::QuotaExceeded, m)
    }
    pub fn file_too_large<T>(m: T) -> Self
    where
        T: Into<StdError>,
    {
        Self::new(ErrorType::FileTooLarge, m)
    }
    pub fn resource_busy<T>(m: T) -> Self
    where
        T: Into<StdError>,
    {
        Self::new(ErrorType::ResourceBusy, m)
    }
    pub fn executable_file_busy<T>(m: T) -> Self
    where
        T: Into<StdError>,
    {
        Self::new(ErrorType::ExecutableFileBusy, m)
    }
    pub fn crosses_devices<T>(m: T) -> Self
    where
        T: Into<StdError>,
    {
        Self::new(ErrorType::CrossesDevices, m)
    }
    pub fn too_many_links<T>(m: T) -> Self
    where
        T: Into<StdError>,
    {
        Self::new(ErrorType::TooManyLinks, m)
    }
    pub fn invalid_filename<T>(m: T) -> Self
    where
        T: Into<StdError>,
    {
        Self::new(ErrorType::InvalidFilename, m)
    }
    pub fn argument_list_too_long<T>(m: T) -> Self
    where
        T: Into<StdError>,
    {
        Self::new(ErrorType::ArgumentListTooLong, m)
    }
    pub fn host_unreachable<T>(m: T) -> Self
    where
        T: Into<StdError>,
    {
        Self::new(ErrorType::HostUnreachable, m)
    }
    pub fn network_unreachable<T>(m: T) -> Self
    where
        T: Into<StdError>,
    {
        Self::new(ErrorType::NetworkUnreachable, m)
    }
    pub fn connection_aborted<T>(m: T) -> Self
    where
        T: Into<StdError>,
    {
        Self::new(ErrorType::ConnectionAborted, m)
    }
    pub fn not_connected<T>(m: T) -> Self
    where
        T: Into<StdError>,
    {
        Self::new(ErrorType::NotConnected, m)
    }
    pub fn network_down<T>(m: T) -> Self
    where
        T: Into<StdError>,
    {
        Self::new(ErrorType::NetworkDown, m)
    }
    pub fn broken_pipe<T>(m: T) -> Self
    where
        T: Into<StdError>,
    {
        Self::new(ErrorType::BrokenPipe, m)
    }
    pub fn directory_not_empty<T>(m: T) -> Self
    where
        T: Into<StdError>,
    {
        Self::new(ErrorType::DirectoryNotEmpty, m)
    }
    pub fn readonly_filesystem<T>(m: T) -> Self
    where
        T: Into<StdError>,
    {
        Self::new(ErrorType::ReadOnlyFilesystem, m)
    }
    pub fn stale_network_file_handle<T>(m: T) -> Self
    where
        T: Into<StdError>,
    {
        Self::new(ErrorType::StaleNetworkFileHandle, m)
    }
    pub fn not_a_directory<T>(m: T) -> Self
    where
        T: Into<StdError>,
    {
        Self::new(ErrorType::NotADirectory, m)
    }
    pub fn is_a_directory<T>(m: T) -> Self
    where
        T: Into<StdError>,
    {
        Self::new(ErrorType::IsADirectory, m)
    }

    pub fn buffer_overflow<T>(m: T) -> Self
    where
        T: Into<StdError>,
    {
        Self::new(ErrorType::BufferOverflow, m)
    }
    pub fn resource_unavailable<T>(m: T) -> Self
    where
        T: Into<StdError>,
    {
        Self::new(ErrorType::ResourceUnavailable, m)
    }
    pub fn file_exists<T>(m: T) -> Self
    where
        T: Into<StdError>,
    {
        Self::new(ErrorType::FileExists, m)
    }
    pub fn other<T>(m: T) -> Self
    where
        T: Into<StdError>,
    {
        Self::new(ErrorType::Other, m)
    }
}

pub type ResultError<T> = Result<T, Error>;

impl From<&str> for Error {
    fn from(s: &str) -> Self {
        Self::new(ErrorType::Other, s)
    }
}

impl From<String> for Error {
    fn from(s: String) -> Self {
        Self::new(ErrorType::Other, s)
    }
}

/// Logs an error with contextual information including the file name and line number.
/// # Examples
/// ```rust
/// use crate::cores::system::catch;
/// fn example_function() -> Result<(), Box<dyn std::error::Error>> {
///      let result: Result<(), &str> = Err("An error occurred");
///     match result {
///         Ok(_) => Ok(()),
///         Err(e) => {
///             catch!(e, "Error in example_function");
///             Err(Box::new(e))
///         }
///     }
/// }
/// ```
#[macro_export]
macro_rules! catch {

    // Match when both error and formatted message with arguments are provided
    ($err:expr, $fmt:expr, $($arg:tt)*) => {{
        let e = $err;
        log::error!("[{}:{}] {}: {}", file!(), line!(), format!($fmt, $($arg)*), e);
        e
    }};

    // Match when error and a simple message are provided
    ($err:expr, $msg:expr) => {{
        let e = $err;
        log::error!("[{}:{}] {}: {}", file!(), line!(), $msg, e);
        e
    }};

    // Match when only the error is provided
    ($err:expr) => {{
        let e = $err;
        log::error!("[{}:{}] {}", file!(), line!(), e);
        e
    }};
}
