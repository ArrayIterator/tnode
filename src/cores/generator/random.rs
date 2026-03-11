use const_format::concatcp;
use rand::{rng, Rng, RngExt};
use std::convert::AsRef;

pub struct Random;

pub const ASCII_ALPHA_UPPER: &str = "ABCDEFGHIJKLMNOPQRSTUVWXYZ";
pub const ASCII_ALPHA_LOWER: &str = "abcdefghijklmnopqrstuvwxyz";
pub const ASCII_ALPHA: &str = concatcp!(ASCII_ALPHA_UPPER, ASCII_ALPHA_LOWER);
pub const ASCII_NUMERIC: &str = "0123456789";
pub const ASCII_SYMBOLS: &str = r#"!@#$%^&*()-_=+[{]}\|;:'",.<>/?`~"#;
pub const ASCII_ALPHANUMERIC: &str = concatcp!(ASCII_ALPHA, ASCII_NUMERIC);
pub const ASCII_FULL: &str = concatcp!(ASCII_ALPHANUMERIC, ASCII_SYMBOLS);
pub const ASCII_HEX: &str = concatcp!("abcdef", ASCII_ALPHANUMERIC);
pub const RFC4648: &str = "ABCDEFGHIJKLMNOPQRSTUVWXYZ234567";

impl Random {
    /// Generate a random string of the specified length using the provided character set.
    /// # Example:
    /// ```rust
    /// use crate::cores::generator::random::Random;
    /// let random_str = Random::random_chars(10, "ABCDEF0123456789");
    /// println!("Random String: {}", random_str);
    /// ```
    pub fn chars<T: AsRef<str>>(length: u32, chars: T) -> String {
        if length < 1 {
            return "".to_string();
        }
        let mut pool: Vec<char> = chars.as_ref().chars().collect();
        if pool.is_empty() {
            pool = ASCII_FULL.chars().collect();
        }

        let mut rng = rand::rng();

        let mut out = String::with_capacity(length as usize);
        for _ in 0..length {
            let idx = rng.random_range(0..pool.len());
            out.push(pool[idx]);
        }
        out
    }

    /// Generate a random alphanumeric string of the specified length
    /// # Example:
    /// ```rust
    /// use crate::cores::generator::random::Random;
    /// let random_str = Random::random_alphanumeric(10);
    /// println!("Random Alphanumeric String: {}", random_str);
    /// ```
    pub fn alphanumeric(length: u32) -> String {
        Self::chars(length, ASCII_ALPHANUMERIC)
    }

    /// Generate a random alpha string of the specified length
    /// # Example:
    /// ```rust
    /// use crate::cores::generator::random::Random;
    /// let random_str = Random::random_alpha(10);
    /// println!("Random Alpha String: {}", random_str);
    /// ```
    pub fn alpha(length: u32) -> String {
        Self::chars(length, ASCII_ALPHA)
    }

    /// Generate a random alpha string of the specified length
    /// # Example:
    /// ```rust
    /// use crate::cores::generator::random::Random;
    /// let random_str = Random::random_alpha_upper(10);
    /// println!("Random Alpha String: {}", random_str);
    /// ```
    pub fn alpha_upper(length: u32) -> String {
        Self::chars(length, ASCII_ALPHA_UPPER)
    }

    /// Generate a random alpha string of the specified length
    /// # Example:
    /// ```rust
    /// use crate::cores::generator::random::Random;
    /// let random_str = Random::random_alpha_lower(10);
    /// println!("Random Alpha String: {}", random_str);
    /// ```
    pub fn alpha_lower(length: u32) -> String {
        Self::chars(length, ASCII_ALPHA_LOWER)
    }

    /// Generate a random alpha string of the specified length
    /// # Example:
    /// ```rust
    /// use crate::cores::generator::random::Random;
    /// let random_str = Random::symbols(10);
    /// println!("Random Alpha String: {}", random_str);
    /// ```
    pub fn symbols(length: u32) -> String {
        Self::chars(length, ASCII_SYMBOLS)
    }

    ///
    /// Generate a random string with symbols of the specified length
    /// # Example:
    /// ```rust
    /// use crate::cores::generator::random::Random;
    /// let random_str = Random::random_string(10);
    /// println!("Random String with Symbols: {}", random_str);
    /// ```
    pub fn string(length: u32) -> String {
        Self::chars(length, ASCII_FULL)
    }

    /// Generate a random hexadecimal string of the specified length
    /// # Example:
    /// ```rust
    /// use crate::cores::generator::random::Random;
    /// let random_hex = Random::random_hex(16);
    /// println!("Random Hexadecimal String: {}", random_hex);
    /// ```
    pub fn hex(length: u32) -> String {
        Self::chars(length, ASCII_HEX)
    }

    /// Generate a random numeric string of the specified length
    /// # Example:
    /// ```rust
    /// use crate::cores::generator::random::Random;
    /// let random_num = Random::random_numeric(10);
    /// println!("Random Numeric String: {}", random_num);
    /// ```
    pub fn number(length: u32) -> String {
        Self::chars(length, ASCII_NUMERIC)
    }

    /// Generate a vector of random bytes of the specified length
    /// # Example:
    /// ```rust
    /// use crate::cores::generator::random::Random;
    /// let random_bytes = Random::bytes(16);
    /// println!("Random Bytes: {:?}", random_bytes);
    /// ```
    pub fn bytes(length: u32) -> Vec<u8> {
        let mut bytes = vec![0u8; length as usize];
        rng().fill_bytes(&mut bytes);
        bytes
    }

    /// Generate a random integer within the specified range (inclusive)
    /// # Example:
    /// ```rust
    /// use crate::cores::generator::random::Random;
    /// let random_int = Random::range(1, 100);
    /// println!("Random Integer: {}", random_int);
    /// ```
    pub fn range(min: i32, max: i32) -> i32 {
        rng().random_range(min..=max)
    }


    /// Generate a random RFC4648 characters
    /// # Example:
    /// ```rust
    /// use crate::cores::generator::random::Random;
    /// let random_rfc4648 = Random::random_rfc4648(100);
    /// println!("Random RFC4648: {}", random_rfc4648);
    /// ```
   pub fn random_rfc4648(length: u32) -> String {
        Self::chars(length, RFC4648)
    }
}
