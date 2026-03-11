use crate::cores::system::error::{Error, ResultError};
use bcrypt::{hash, verify};
use log::debug;
use regex::Regex;
use std::sync::LazyLock;

pub const MAXIMUM_COST: u32 = 20;
pub const DEFAULT_COST: u32 = 12;
pub const MINIMUM_COST: u32 = 4;

//static BCRYPT_REGEX: LazyLock<Regex> =
//    LazyLock::new(|| Regex::new(r"^\$2[ayb]\$[0-9]{2}\$[./A-Za-z0-9]{53}$").unwrap());
static BCRYPT_REGEX_PREFIX: LazyLock<Regex> = LazyLock::new(|| {
    Regex::new(r"^(?:\$(?P<prefix>[^$]+)\$)?(?P<hash>\$2(?P<variant>[ayb])\$(?P<cost>[0-9]{2})\$[./A-Za-z0-9]{53})$")
        .unwrap()
});

#[derive(Debug, Clone)]
pub struct BcryptDetail {
    pub prefix: Option<String>,
    pub variant_identifier: String,
    pub cost: u32,
    pub hash: String,
    pub full: String,
}

pub struct Password;

/// Password Helper that implements methods to hash and verify passwords using bcrypt.
/// The password method using bcrypt algorithm.
/// Implementation
impl Password {
    /// Check if a string is a valid bcrypt hash
    /// # Example:
    /// ```rust
    /// use crate::cores::auth::password::Password;
    /// let is_valid = Password::valid("$2b$12$KIXQJE6h1G8Z8e1Y5Z6OeuFhFhJ8e1Y5Z6OeuFhFhJ8e1Y5Z6Oeu");
    /// println!("Is valid bcrypt hash: {}", is_valid);
    /// ```
    pub fn valid<T: AsRef<str>>(hash: T) -> bool {
        Self::parse(hash).is_ok()
    }

    /// Parse bcrypt (prefix support)
    /// # Example
    /// ```rust
    /// use crate::cores::auth::password::Password;
    /// match Password::parse("$wp$2y$10$Wk5wfMp9NotTWa1pgs2KguBqS4Xcc00MS.3QRNC1RBK5LQ0bwESi.") {
    ///     Ok(parsed) => {
    ///         println("Parsed: {:?}", parsed);
    ///         assert_eq!(&parsed.prefix.unwrap_or(||"".to_string()), "wp")
    ///     }
    ///     Err(e) => {
    ///         log::error(e)
    ///     }
    /// }
    /// ```
    pub fn parse<T: AsRef<str>>(full_hash: T) -> ResultError<BcryptDetail> {
        let full_hash = full_hash.as_ref();
        if full_hash.len() < 60 {
            return Err(Error::invalid_length("Length of has is not satisfied"));
        }
        let full = full_hash.to_string();
        if let Some(matches) = BCRYPT_REGEX_PREFIX.captures(full_hash) {
            let hash = matches
                .name("hash")
                .map(|e| e.as_str().to_string())
                .ok_or_else(|| Error::parse_error("Hash is contain invalid hash!"))?;
            if hash.len() != 60 {
                return Err(Error::invalid_length("Has length is invalid"));
            }
            let variant_identifier = matches
                .name("variant")
                .map(|e| e.as_str().to_string())
                .ok_or_else(|| Error::parse_error("Invalid bcrypt variant"))?;
            let cost = matches
                .name("cost")
                .map(|e| e.as_str().to_string())
                .ok_or_else(|| Error::parse_error("Hash is contain invalid cost!"))?
                .parse::<u32>()
                .map_err(|e| Error::parse_error("Hash contain invalid cost"))?;
            let prefix = matches.name("prefix").map(|e| e.as_str().to_string());
            return Ok(BcryptDetail {
                prefix,
                variant_identifier,
                cost,
                hash,
                full,
            });
        }
        Err(Error::parse_error("Hash is invalid"))
    }

    /// Hash a password using bcrypt with the default cost
    /// # Example:
    /// ```rust
    /// use crate::cores::auth::password::Password;
    /// let hashed = Password::hash("my_secure_password").unwrap();
    /// println!("Hashed password: {}", hashed);
    /// ```
    pub fn hash<T: AsRef<str>>(password: T) -> ResultError<String> {
        hash(password.as_ref(), DEFAULT_COST).map_err(|e| Error::from_error(e))
    }

    /// Hash a password using bcrypt with a specified cost
    /// # Example:
    /// ```rust
    /// use crate::cores::auth::password::Password;
    /// let hashed = Password::hash_cost("my_secure_password", 14).unwrap();
    /// println!("Hashed password with cost 14: {}", hashed);
    /// ```
    pub fn hash_cost<T: AsRef<str>>(password: T, cost: u32) -> ResultError<String> {
        if cost < MINIMUM_COST {
            return Err(Error::invalid_range(format!(
                "Cost must be greater or equal than {}, {} given",
                MINIMUM_COST, cost
            )));
        }
        if cost > MAXIMUM_COST {
            return Err(Error::invalid_range(format!(
                "Cost must be less or equal than {}, {} given",
                MAXIMUM_COST, cost
            )));
        }
        hash(password.as_ref(), cost).map_err(|e| Error::from_error(e))
    }

    /// Verify a password against a stored bcrypt hash
    /// # Example:
    /// ```rust
    /// use crate::cores::auth::password::Password;
    /// let stored_hash = Password::hash("my_secure_password").unwrap();
    /// let is_valid = Password::verify("my_secure_password", &stored_hash);
    /// println!("Password is valid: {}", is_valid);
    /// ```
    pub fn verify<T: AsRef<str>, H: AsRef<str>>(password: T, stored_hash: H) -> bool {
        match Self::parse(stored_hash) {
            Ok(det) => verify(password.as_ref(), &det.hash).unwrap_or_else(|e| {
                debug!(target: "auth:bcrypt", "Bcrypt verification error: {}", e);
                false
            }),
            Err(e) => {
                debug!(target: "auth:bcrypt", "Bcrypt parse error: {}", e);
                false
            }
        }
    }
}
