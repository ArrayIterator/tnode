use crate::cores::system::error::{Error, ResultError};
use serde::{Deserialize, Deserializer, Serialize, Serializer};

/// Represents HTTP status codes as defined in the HTTP/1.1 standard. Each variant corresponds to
/// a specific status code grouped by its class (1xx Informational, 2xx Success, 3xx Redirection,
/// 4xx Client Errors, and 5xx Server Errors).
///
/// # Attributes
/// Each variant has an associated numeric status code as its value, represented as a `u16`.
///
/// # Examples
///
/// ```
/// use crate::HttpStatus;
///
/// let status = HttpStatus::Ok;
/// match status {
///     HttpStatus::Ok => println!("Request succeeded with status code {:?}", status as u16),
///     HttpStatus::NotFound => println!("Resource not found"),
///     _ => println!("Other status"),
/// }
/// ```
///
/// ## Variants
/// ### 1xx Informational
/// - `Continue`: Status code 100
/// - `SwitchingProtocols`: Status code 101
/// - `Processing`: Status code 102
///
/// ### 2xx Success
/// - `Ok`: Status code 200
/// - `Created`: Status code 201
/// - `Accepted`: Status code 202
/// - `NonAuthoritativeInformation`: Status code 203
/// - `NoContent`: Status code 204
/// - `ResetContent`: Status code 205
/// - `PartialContent`: Status code 206
/// - `MultiStatus`: Status code 207
/// - `AlreadyReported`: Status code 208
///
/// ### 3xx Redirection
/// - `MultipleChoices`: Status code 300
/// - `MovedPermanently`: Status code 301
/// - `Found`: Status code 302
/// - `SeeOther`: Status code 303
/// - `NotModified`: Status code 304
/// - `UseProxy`: Status code 305
/// - `SwitchProxy`: Status code 306
/// - `TemporaryRedirect`: Status code 307
///
/// ### 4xx Client Errors
/// - `BadRequest`: Status code 400
/// - `Unauthorized`: Status code 401
/// - `PaymentRequired`: Status code 402
/// - `Forbidden`: Status code 403
/// - `NotFound`: Status code 404
/// - `MethodNotAllowed`: Status code 405
/// - `NotAcceptable`: Status code 406
/// - `ProxyAuthenticationRequired`: Status code 407
/// - `RequestTimeout`: Status code 408
/// - `Conflict`: Status code 409
/// - `Gone`: Status code 410
/// - `LengthRequired`: Status code 411
/// - `PreconditionFailed`: Status code 412
/// - `RequestEntityTooLarge`: Status code 413
/// - `RequestUriTooLarge`: Status code 414
/// - `UnsupportedMediaType`: Status code 415
/// - `RequestedRangeNotSatisfiable`: Status code 416
/// - `ExpectationFailed`: Status code 417
/// - `ImATeapot`: Status code 418 (Easter egg status)
/// - `UnprocessableEntity`: Status code 422
/// - `Locked`: Status code 423
/// - `FailedDependency`: Status code 424
/// - `UnorderedCollection`: Status code 425 (Unofficial)
/// - `UpgradeRequired`: Status code 426
/// - `PreconditionRequired`: Status code 428
/// - `TooManyRequests`: Status code 429
/// - `RequestHeaderFieldsTooLarge`: Status code 431
/// - `UnavailableForLegalReasons`: Status code 451
///
/// ### 5xx Server Errors
/// - `InternalServerError`: Status code 500
/// - `NotImplemented`: Status code 501
/// - `BadGateway`: Status code 502
/// - `ServiceUnavailable`: Status code 503
/// - `GatewayTimeout`: Status code 504
/// - `HttpVersionNotSupported`: Status code 505
/// - `VariantAlsoNegotiates`: Status code 506
/// - `InsufficientStorage`: Status code 507
/// - `LoopDetected`: Status code 508
/// - `NetworkAuthenticationRequired`: Status code 511
///
/// # Derives
/// This enum derives the following traits:
/// - `Debug`: Enables formatting with the `{:?}` formatter.
/// - `Clone`: Allows the enum to be cloned.
/// - `Copy`: Provides copy semantics.
/// - `PartialEq` and `Eq`: Enables comparison of enum variants.
/// - `Hash`: Allows the enum to be used as a key in hashed collections.
///
/// # Repr
/// This enum uses `#[repr(u16)]`, ensuring that each variant is stored as a `u16`.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
#[repr(u16)]
pub enum HttpStatus {
    // 1xx Informational
    Continue = 100,
    SwitchingProtocols = 101,
    Processing = 102,

    // 2xx Success
    Ok = 200,
    Created = 201,
    Accepted = 202,
    NonAuthoritativeInformation = 203,
    NoContent = 204,
    ResetContent = 205,
    PartialContent = 206,
    MultiStatus = 207,
    AlreadyReported = 208,

    // 3xx Redirection
    MultipleChoices = 300,
    MovedPermanently = 301,
    Found = 302,
    SeeOther = 303,
    NotModified = 304,
    UseProxy = 305,
    SwitchProxy = 306,
    TemporaryRedirect = 307,

    // 4xx Client Errors
    BadRequest = 400,
    Unauthorized = 401,
    PaymentRequired = 402,
    Forbidden = 403,
    NotFound = 404,
    MethodNotAllowed = 405,
    NotAcceptable = 406,
    ProxyAuthenticationRequired = 407,
    RequestTimeout = 408,
    Conflict = 409,
    Gone = 410,
    LengthRequired = 411,
    PreconditionFailed = 412,
    RequestEntityTooLarge = 413,
    RequestUriTooLarge = 414,
    UnsupportedMediaType = 415,
    RequestedRangeNotSatisfiable = 416,
    ExpectationFailed = 417,
    ImATeapot = 418,
    UnprocessableEntity = 422,
    Locked = 423,
    FailedDependency = 424,
    UnorderedCollection = 425,
    UpgradeRequired = 426,
    PreconditionRequired = 428,
    TooManyRequests = 429,
    RequestHeaderFieldsTooLarge = 431,
    UnavailableForLegalReasons = 451,

    // 5xx Server Errors
    InternalServerError = 500,
    NotImplemented = 501,
    BadGateway = 502,
    ServiceUnavailable = 503,
    GatewayTimeout = 504,
    HttpVersionNotSupported = 505,
    VariantAlsoNegotiates = 506,
    InsufficientStorage = 507,
    LoopDetected = 508,
    NetworkAuthenticationRequired = 511,
}

impl Serialize for HttpStatus {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer
    {
        serializer.serialize_u16(self.to_u16())
    }
}

impl<'de> Deserialize<'de> for HttpStatus {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: Deserializer<'de>
    {
        Self::from_u16(u16::deserialize(deserializer)?).map_err(serde::de::Error::custom)
    }
}

impl HttpStatus {
    /// Converts the value of the implementing type into a `u16`.
    ///
    /// This method casts the current value (`self`) to a `u16` using a dereference,
    /// assuming the implementing type can safely be represented as a `u16` without loss of data.
    ///
    /// # Returns
    ///
    /// A `u16` representing the value of the implementing type.
    ///
    /// # Examples
    ///
    /// ```rust
    /// let value: YourType = YourType::new();
    /// let converted: u16 = value.to_u16();
    /// println!("Converted value: {}", converted);
    /// ```
    ///
    /// # Panics
    ///
    /// This method does not perform any bounds checking, so behavior is undefined
    /// if the value cannot be safely cast to `u16`.
    pub fn to_u16(&self) -> u16 {
        *self as u16
    }

    /// Returns the reason phrase associated with the `HttpStatus` variant.
    ///
    /// This method maps each HTTP status code represented by the `HttpStatus` enum
    /// to a corresponding human-readable textual description, as defined by the
    /// HTTP standard. These reason phrases provide additional context for the
    /// HTTP status code and are commonly used in HTTP responses.
    ///
    /// # Returns
    /// * A string slice (`&'static str`) containing the reason phrase for each `HttpStatus` variant.
    ///
    /// # Example
    /// ```rust
    /// let status = HttpStatus::Ok;
    /// assert_eq!(status.reason_phrase(), "OK");
    /// ```
    ///
    /// # Notes
    /// - This method is a static association between enum variants and their reason phrases.
    ///   It does not provide any dynamic behavior.
    /// - The mapping is based on standard HTTP status codes as per relevant RFC specifications.
    ///
    /// # Enum Variants and Corresponding Reason Phrases
    /// - `HttpStatus::Continue` -> "Continue"
    /// - `HttpStatus::SwitchingProtocols` -> "Switching Protocols"
    /// - `HttpStatus::Processing` -> "Processing"
    /// - `HttpStatus::Ok` -> "OK"
    /// - `HttpStatus::Created` -> "Created"
    /// - `HttpStatus::Accepted` -> "Accepted"
    /// - `HttpStatus::NonAuthoritativeInformation` -> "Non-Authoritative Information"
    /// - `HttpStatus::NoContent` -> "No Content"
    /// - `HttpStatus::ResetContent` -> "Reset Content"
    /// - `HttpStatus::PartialContent` -> "Partial Content"
    /// - `HttpStatus::MultiStatus` -> "Multi-status"
    /// - `HttpStatus::AlreadyReported` -> "Already Reported"
    /// - `HttpStatus::MultipleChoices` -> "Multiple Choices"
    /// - `HttpStatus::MovedPermanently` -> "Moved Permanently"
    /// - `HttpStatus::Found` -> "Found"
    /// - `HttpStatus::SeeOther` -> "See Other"
    /// - `HttpStatus::NotModified` -> "Not Modified"
    /// - `HttpStatus::UseProxy` -> "Use Proxy"
    /// - `HttpStatus::SwitchProxy` -> "Switch Proxy"
    /// - `HttpStatus::TemporaryRedirect` -> "Temporary Redirect"
    /// - `HttpStatus::BadRequest` -> "Bad Request"
    /// - `HttpStatus::Unauthorized` -> "Unauthorized"
    /// - `HttpStatus::PaymentRequired` -> "Payment Required"
    /// - `HttpStatus::Forbidden` -> "Forbidden"
    /// - `HttpStatus::NotFound` -> "Not Found"
    /// - `HttpStatus::MethodNotAllowed` -> "Method Not Allowed"
    /// - `HttpStatus::NotAcceptable` -> "Not Acceptable"
    /// - `HttpStatus::ProxyAuthenticationRequired` -> "Proxy Authentication Required"
    /// - `HttpStatus::RequestTimeout` -> "Request Time-out"
    /// - `HttpStatus::Conflict` -> "Conflict"
    /// - `HttpStatus::Gone` -> "Gone"
    /// - `HttpStatus::LengthRequired` -> "Length Required"
    /// - `HttpStatus::PreconditionFailed` -> "Precondition Failed"
    /// - `HttpStatus::RequestEntityTooLarge` -> "Request Entities Too Large"
    /// - `HttpStatus::RequestUriTooLarge` -> "Request-URI Too Large"
    /// - `HttpStatus::UnsupportedMediaType` -> "Unsupported Media Type"
    /// - `HttpStatus::RequestedRangeNotSatisfiable` -> "Requested range not satisfiable"
    /// - `HttpStatus::ExpectationFailed` -> "Expectation Failed"
    /// - `HttpStatus::ImATeapot` -> "I'm a teapot"
    /// - `HttpStatus::UnprocessableEntity` -> "Unprocessable Entities"
    /// - `HttpStatus::Locked` -> "Locked"
    /// - `HttpStatus::FailedDependency` -> "Failed Dependency"
    /// - `HttpStatus::UnorderedCollection` -> "Unordered Collection"
    /// - `HttpStatus::UpgradeRequired` -> "Upgrade Required"
    /// - `HttpStatus::PreconditionRequired` -> "Precondition Required"
    /// - `HttpStatus::TooManyRequests` -> "Too Many Requests"
    /// - `HttpStatus::RequestHeaderFieldsTooLarge` -> "Request Header Fields Too Large"
    /// - `HttpStatus::UnavailableForLegalReasons` -> "Unavailable For Legal Reasons"
    /// - `HttpStatus::InternalServerError` -> "Internal Server Error"
    /// - `HttpStatus::NotImplemented` -> "Not Implemented"
    /// - `HttpStatus::BadGateway` -> "Bad Gateway"
    /// - `HttpStatus::ServiceUnavailable` -> "Service Unavailable"
    /// - `HttpStatus::GatewayTimeout` -> "Gateway Time-out"
    /// - `HttpStatus::HttpVersionNotSupported` -> "HTTP Version not supported"
    /// - `HttpStatus::VariantAlsoNegotiates` -> "Variant Also Negotiates"
    /// - `HttpStatus::InsufficientStorage` -> "Insufficient Storage"
    /// - `HttpStatus::LoopDetected` -> "Loop Detected"
    /// - `HttpStatus::NetworkAuthenticationRequired` -> "Network Authentication Required"
    pub fn reason_phrase(&self) -> &'static str {
        match self {
            HttpStatus::Continue => "Continue",
            HttpStatus::SwitchingProtocols => "Switching Protocols",
            HttpStatus::Processing => "Processing",
            HttpStatus::Ok => "OK",
            HttpStatus::Created => "Created",
            HttpStatus::Accepted => "Accepted",
            HttpStatus::NonAuthoritativeInformation => "Non-Authoritative Information",
            HttpStatus::NoContent => "No Content",
            HttpStatus::ResetContent => "Reset Content",
            HttpStatus::PartialContent => "Partial Content",
            HttpStatus::MultiStatus => "Multi-status",
            HttpStatus::AlreadyReported => "Already Reported",
            HttpStatus::MultipleChoices => "Multiple Choices",
            HttpStatus::MovedPermanently => "Moved Permanently",
            HttpStatus::Found => "Found",
            HttpStatus::SeeOther => "See Other",
            HttpStatus::NotModified => "Not Modified",
            HttpStatus::UseProxy => "Use Proxy",
            HttpStatus::SwitchProxy => "Switch Proxy",
            HttpStatus::TemporaryRedirect => "Temporary Redirect",
            HttpStatus::BadRequest => "Bad Request",
            HttpStatus::Unauthorized => "Unauthorized",
            HttpStatus::PaymentRequired => "Payment Required",
            HttpStatus::Forbidden => "Forbidden",
            HttpStatus::NotFound => "Not Found",
            HttpStatus::MethodNotAllowed => "Method Not Allowed",
            HttpStatus::NotAcceptable => "Not Acceptable",
            HttpStatus::ProxyAuthenticationRequired => "Proxy Authentication Required",
            HttpStatus::RequestTimeout => "Request Time-out",
            HttpStatus::Conflict => "Conflict",
            HttpStatus::Gone => "Gone",
            HttpStatus::LengthRequired => "Length Required",
            HttpStatus::PreconditionFailed => "Precondition Failed",
            HttpStatus::RequestEntityTooLarge => "Request Entities Too Large",
            HttpStatus::RequestUriTooLarge => "Request-URI Too Large",
            HttpStatus::UnsupportedMediaType => "Unsupported Media Type",
            HttpStatus::RequestedRangeNotSatisfiable => "Requested range not satisfiable",
            HttpStatus::ExpectationFailed => "Expectation Failed",
            HttpStatus::ImATeapot => "I'm a teapot",
            HttpStatus::UnprocessableEntity => "Unprocessable Entities",
            HttpStatus::Locked => "Locked",
            HttpStatus::FailedDependency => "Failed Dependency",
            HttpStatus::UnorderedCollection => "Unordered Collection",
            HttpStatus::UpgradeRequired => "Upgrade Required",
            HttpStatus::PreconditionRequired => "Precondition Required",
            HttpStatus::TooManyRequests => "Too Many Requests",
            HttpStatus::RequestHeaderFieldsTooLarge => "Request Header Fields Too Large",
            HttpStatus::UnavailableForLegalReasons => "Unavailable For Legal Reasons",
            HttpStatus::InternalServerError => "Internal Server Error",
            HttpStatus::NotImplemented => "Not Implemented",
            HttpStatus::BadGateway => "Bad Gateway",
            HttpStatus::ServiceUnavailable => "Service Unavailable",
            HttpStatus::GatewayTimeout => "Gateway Time-out",
            HttpStatus::HttpVersionNotSupported => "HTTP Version not supported",
            HttpStatus::VariantAlsoNegotiates => "Variant Also Negotiates",
            HttpStatus::InsufficientStorage => "Insufficient Storage",
            HttpStatus::LoopDetected => "Loop Detected",
            HttpStatus::NetworkAuthenticationRequired => "Network Authentication Required",
        }
    }

    pub fn from_u16(code: u16) -> ResultError<HttpStatus> {
        match code {
            100 => Ok(HttpStatus::Continue),
            101 => Ok(HttpStatus::SwitchingProtocols),
            102 => Ok(HttpStatus::Processing),
            200 => Ok(HttpStatus::Ok),
            201 => Ok(HttpStatus::Created),
            202 => Ok(HttpStatus::Accepted),
            203 => Ok(HttpStatus::NonAuthoritativeInformation),
            204 => Ok(HttpStatus::NoContent),
            205 => Ok(HttpStatus::ResetContent),
            206 => Ok(HttpStatus::PartialContent),
            207 => Ok(HttpStatus::MultiStatus),
            208 => Ok(HttpStatus::AlreadyReported),
            300 => Ok(HttpStatus::MultipleChoices),
            301 => Ok(HttpStatus::MovedPermanently),
            302 => Ok(HttpStatus::Found),
            303 => Ok(HttpStatus::SeeOther),
            304 => Ok(HttpStatus::NotModified),
            305 => Ok(HttpStatus::UseProxy),
            306 => Ok(HttpStatus::SwitchProxy),
            307 => Ok(HttpStatus::TemporaryRedirect),
            400 => Ok(HttpStatus::BadRequest),
            401 => Ok(HttpStatus::Unauthorized),
            402 => Ok(HttpStatus::PaymentRequired),
            403 => Ok(HttpStatus::Forbidden),
            404 => Ok(HttpStatus::NotFound),
            405 => Ok(HttpStatus::MethodNotAllowed),
            406 => Ok(HttpStatus::NotAcceptable),
            407 => Ok(HttpStatus::ProxyAuthenticationRequired),
            408 => Ok(HttpStatus::RequestTimeout),
            409 => Ok(HttpStatus::Conflict),
            410 => Ok(HttpStatus::Gone),
            411 => Ok(HttpStatus::LengthRequired),
            412 => Ok(HttpStatus::PreconditionFailed),
            413 => Ok(HttpStatus::RequestEntityTooLarge),
            414 => Ok(HttpStatus::RequestUriTooLarge),
            415 => Ok(HttpStatus::UnsupportedMediaType),
            416 => Ok(HttpStatus::RequestedRangeNotSatisfiable),
            417 => Ok(HttpStatus::ExpectationFailed),
            418 => Ok(HttpStatus::ImATeapot),
            422 => Ok(HttpStatus::UnprocessableEntity),
            423 => Ok(HttpStatus::Locked),
            424 => Ok(HttpStatus::FailedDependency),
            425 => Ok(HttpStatus::UnorderedCollection),
            426 => Ok(HttpStatus::UpgradeRequired),
            428 => Ok(HttpStatus::PreconditionRequired),
            429 => Ok(HttpStatus::TooManyRequests),
            431 => Ok(HttpStatus::RequestHeaderFieldsTooLarge),
            451 => Ok(HttpStatus::UnavailableForLegalReasons),
            500 => Ok(HttpStatus::InternalServerError),
            501 => Ok(HttpStatus::NotImplemented),
            502 => Ok(HttpStatus::BadGateway),
            503 => Ok(HttpStatus::ServiceUnavailable),
            504 => Ok(HttpStatus::GatewayTimeout),
            505 => Ok(HttpStatus::HttpVersionNotSupported),
            506 => Ok(HttpStatus::VariantAlsoNegotiates),
            507 => Ok(HttpStatus::InsufficientStorage),
            508 => Ok(HttpStatus::LoopDetected),
            511 => Ok(HttpStatus::NetworkAuthenticationRequired),
            _ => Err(Error::invalid_input(format!("Invalid HTTP status code: {}", code))),
        }
    }
}
