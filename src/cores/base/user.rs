use crate::cores::auth::password::Password;
use crate::cores::generator::random::{Random, ASCII_SYMBOLS};
use crate::cores::idna::domain::Domain;
use crate::cores::system::error::{Error, ResultError};
use rand::prelude::SliceRandom;

/// A trait that represents the basic operations and attributes associated with a user.
///
/// This trait is intended to be implemented by user-related structures to provide
/// unique identification and a username.
///
/// # Required Methods
///
/// ## `id`
///
/// ```rust
/// fn id(&self) -> u64;
/// ```
///
/// Retrieves the unique identifier for the user.
///
/// **Returns:**
/// - `u64`: A unique, non-negative integer representing the user's ID.
///
/// ## `username`
///
/// ```rust
/// fn username(&self) -> String;
/// ```
///
/// Retrieves the username of the user.
///
/// **Returns:**
/// - `String`: A string representing the username of the user.
///
/// # Examples
///
/// ```rust
/// struct User {
///     user_id: u64,
///     name: String,
/// }
///
/// impl UserBase for User {
///     fn id(&self) -> u64 {
///         self.user_id
///     }
///
///     fn username(&self) -> String {
///         self.name.clone()
///     }
/// }
///
/// let user = User {
///     user_id: 1,
///     name: "JohnDoe".to_string(),
/// };
///
/// assert_eq!(user.id(), 1);
/// assert_eq!(user.username(), "JohnDoe".to_string());
/// ```
pub trait UserBase {
    /// Retrieves the unique identifier associated with the implementing object.
    ///
    /// # Returns
    /// * `i64` - A 64-bit integer representing the unique identifier.
    ///
    /// # Examples
    /// ```
    /// struct Item {
    ///     id: i64,
    /// }
    ///
    /// impl Item {
    ///     fn id(&self) -> i64 {
    ///         self.id
    ///     }
    /// }
    ///
    /// let item = Item { id: 42 };
    /// assert_eq!(item.id(), 42);
    /// ```
    fn id(&self) -> i64;

    /// Retrieves the username associated with the current instance.
    ///
    /// This method returns a `String` representing the username. It is expected
    /// to be implemented by types that include user-related information.
    ///
    /// # Returns
    /// * `String` - The username as a string.
    ///
    /// # Examples
    /// ```
    /// let user = User::new("Alice");
    /// let username = user.username();
    /// assert_eq!(username, "Alice".to_string());
    /// ```
    fn username(&self) -> &str;

    /// Retrieves the email associated with the current instance.
    ///
    /// This method returns a `String` representing the email. It is expected
    /// to be implemented by types that include user-related information.
    ///
    /// # Returns
    /// * `String` - The username as a string.
    ///
    /// # Examples
    /// ```
    /// let user = User::new("alice@example.com");
    /// let username = user.email();
    /// assert_eq!(username, "alice@example.com".to_string());
    /// ```
    fn email(&self) -> &str;

    /// Retrieves the password associated with the current instance.
    ///
    /// # Returns
    ///
    /// * `&str` - A string slice representing the password.
    ///
    /// # Note
    ///
    /// This function provides read-only access to the password. Ensure proper handling
    /// of sensitive data to maintain security and confidentiality.
    ///
    /// # Example
    ///
    /// ```rust
    /// struct User {
    ///     password: String,
    /// }
    ///
    /// impl User {
    ///     fn password(&self) -> &str {
    ///         &self.password
    ///     }
    /// }
    ///
    /// let user = User {
    ///     password: String::from("secure_password"),
    /// };
    ///
    /// assert_eq!(user.password(), "secure_password");
    /// ```
    fn password(&self) -> &str;

    /// This method compares the given password against the user's stored password
    /// using a secure verification method.
    ///
    /// # Arguments
    ///
    /// * `password` - A string slice representing the password to verify.
    ///
    /// # Returns
    ///
    /// * `bool` - `true` if the password matches, `false` otherwise.
    fn verify_password(&self, password: &str) -> bool {
        Password::verify(password, self.password())
    }
}

pub const MIN_USERNAME_LENGTH: usize = 3;
pub const MAX_USERNAME_LENGTH: usize = 40;

pub struct Util;

impl Util {
    pub fn filter_username<T: AsRef<str>>(username: T) -> ResultError<String> {
        let username = username.as_ref().trim().to_string().to_lowercase();
        if username.is_empty() {
            return Err(Error::invalid_input("Username could not be empty"));
        }
        let len = username.len();
        if len > crate::app::models::user::MAX_USERNAME_LENGTH {
            return Err(Error::invalid_length(format!(
                "Username too long! maximum is  {} characters.",
                crate::app::models::user::MAX_USERNAME_LENGTH
            )));
        }
        if len < crate::app::models::user::MIN_USERNAME_LENGTH {
            return Err(Error::invalid_length(format!(
                "Username too short! minimum is {} characters",
                crate::app::models::user::MIN_USERNAME_LENGTH
            )));
        }
        Ok(username)
    }
    pub fn filter_email<T: AsRef<str>>(email: T) -> ResultError<String> {
        let email = Domain::parse_email(email)?;
        Ok(email.to_lowercase())
    }

    pub fn is_strong_password<T: AsRef<str>>(password: T) -> bool {
        let password = password.as_ref();
        if password.len() < 8 {
            return false;
        }

        let mut has_upper = false;
        let mut has_lower = false;
        let mut has_numeric = false;
        let mut has_special = false;
        for c in password.chars() {
            if c.is_ascii_uppercase() {
                has_upper = true;
            } else if c.is_ascii_lowercase() {
                has_lower = true;
            } else if c.is_ascii_digit() {
                has_numeric = true;
            } else if ASCII_SYMBOLS.contains(c) {
                has_special = true;
            }
            if has_upper && has_lower && has_numeric && has_special {
                return true;
            }
        }
        has_upper && has_lower && has_numeric && has_special
    }

    pub fn gen_password(length: usize) -> String {
        let length = length.clamp(4, u32::MAX as usize) as u32;
        let mut password: Vec<char> = Vec::with_capacity(length as usize);
        password.push(Random::alpha_upper(1).chars().next().unwrap());
        password.push(Random::alpha_lower(1).chars().next().unwrap());
        password.push(Random::number(1).chars().next().unwrap());
        password.push(Random::symbols(1).chars().next().unwrap());
        let remaining = length - 4;
        if remaining > 0 {
            let rest = Random::string(remaining);
            password.extend(rest.chars());
        }

        let mut rng = rand::rng();
        password.shuffle(&mut rng);
        password.into_iter().collect()
    }
}
