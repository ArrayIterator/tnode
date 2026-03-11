use crate::cores::base::user::UserBase;
use crate::cores::helper::hack::ToHex;
use crate::cores::helper::hash::{HashTraitFixed, HmacSha256};
use crate::cores::system::error::{Error, ResultError};
use hmac::Mac;
use std::fmt::Display;
use std::time::Duration;
use uuid::Uuid;

/// Represents the verified data structure extracted from a session token.
#[derive(Clone, Debug)]
pub struct SessionPayload {
    pub(crate) token: String,
    /// The unique numeric identifier of the USER.
    pub(crate) user_id: u64,
    /// The UNIX timestamp representing the token generation time.
    pub(crate) timestamp: i64,
    /// A 16-byte random nonce for cryptographic uniqueness (derived from UUID v4).
    pub(crate) random_16: [u8; 16],
}

impl SessionPayload {
    pub fn token(&self) -> &str {
        &self.token
    }
    pub fn user_id(&self) -> u64 {
        self.user_id
    }
    pub fn timestamp(&self) -> i64 {
        self.timestamp
    }
    pub fn random(&self) -> &[u8; 16] {
        &self.random_16
    }
    pub fn is_expired_with(&self, duration: Duration) -> bool {
        if self.timestamp <= 0 {
            return true;
        }
        let now = chrono::Utc::now().timestamp() as u64;
        let timestamp = (self.timestamp as u64);
        if timestamp >= now { // future fail
            return true;
        }
        let age_secs = now - timestamp;
        age_secs <= 0 || age_secs >= duration.as_secs()
    }
}

impl Display for SessionPayload {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.token())
    }
}

/// A stateless session tokenizer providing integrity protection via nested HMAC-SHA256.
///
/// This tokenizer ensures that session data is both authentic and tamper-proof
/// without requiring a centralized database for session lookup.
#[derive(Clone, Debug)]
pub struct SessionTokenizer {
    secret: Vec<u8>,
    salt: Vec<u8>,
}

impl SessionTokenizer {
    pub const LENGTH: usize = 192;

    /// Creates a new instance of the struct with the provided `secret` and `salt`.
    ///
    /// # Type Parameters
    /// - `Secret`: A type that can be converted to a string slice (`&str`).
    /// - `Salt`: A type that can be converted to a string slice (`&str`).
    ///
    /// # Parameters
    /// - `secret`: A value implementing `AsRef<str>` which represents the secret key.
    /// - `salt`: A value implementing `AsRef<str>` which represents the salt.
    ///
    /// # Returns
    /// A new instance of the struct with the `secret` and `salt` converted to byte vectors (`Vec<u8>`).
    ///
    /// # Examples
    /// ```
    /// let secret = "my_secret";
    /// let salt = "my_salt";
    /// let instance = MyStruct::new(secret, salt);
    /// ```
    pub fn new<Secret: AsRef<str>, Salt: AsRef<str>>(secret: Secret, salt: Salt) -> Self {
        let secret = secret.as_ref();
        let salt = salt.as_ref();
        Self {
            secret: secret.as_bytes().to_vec(),
            salt: salt.as_bytes().to_vec(),
        }
    }

    /// Combines the secret and salt slices into a single `Vec<u8>`.
    ///
    /// # Description
    /// This function concatenates the byte slices of `self.secret` and `self.salt`
    /// to produce a combined vector. This is useful for cases where a derived
    /// key or unique identifier is needed by merging the two values.
    ///
    /// # Returns
    /// A `Vec<u8>` containing the combined byte data of `self.secret` and `self.salt`.
    ///
    /// # Example
    /// ```rust
    /// let instance = MyStruct {
    ///     secret: vec![1, 2, 3],
    ///     salt: vec![4, 5, 6],
    /// };
    /// let result = instance.get_combined_key();
    /// assert_eq!(result, vec![1, 2, 3, 4, 5, 6]);
    /// ```
    fn get_combined_key(&self) -> Vec<u8> {
        [self.secret.as_slice(), self.salt.as_slice()].concat()
    }

    /// Generates a secure, cryptographically signed token for a given user.
    ///
    /// # Parameters
    /// - `user_id`: A 64-bit unsigned integer representing the user's unique identifier.
    ///
    /// # Returns
    /// - `Ok(SessionPayload)`: A signed, payload hex-encoded token string on success.
    /// - `Err(ResultError<String>)`: An error object in case of failure during the token generation process.
    ///
    /// # Token Structure
    /// The generated token is a 96-byte buffer, consisting of the following components:
    /// 1. Bytes 0..16: A random 16-byte value generated using a UUID v4.
    /// 2. Bytes 16..48: An HMAC-SHA256 binding of `user_id` with a secret key.
    /// 3. Bytes 48..56: An 8-byte representation of the current UNIX timestamp in seconds (big-endian).
    /// 4. Bytes 56..64: An 8-byte representation of the `user_id` (big-endian).
    /// 5. Bytes 64..96: An HMAC-SHA256 signature of the first 64 bytes, using the same secret key.
    ///
    /// # Implementation Details
    /// - The secret key is fetched via the `get_combined_key` method.
    /// - Unique randomness for the token is achieved using a UUID v4.
    /// - The function uses HMAC-SHA256 for cryptographic binding and signing.
    /// - The timestamp ensures the token is time-sensitive and can be validated based on issuance time.
    /// - The resulting 96-byte buffer is hex-encoded into a string for the final output.
    ///
    /// # Errors
    /// This function can return an error in the following cases:
    /// - Failure in HMAC-SHA256 computation during `id_binding` or `final_sign`.
    /// - Any other unexpected failure during token composition.
    ///
    /// # Example
    /// ```rust
    /// let service = TokenService::new();
    /// let user_id = 123456789;
    /// match service.generate(user_id) {
    ///     Ok(token) => println!("Generated token: {}", token),
    ///     Err(err) => eprintln!("Token generation failed: {}", err),
    /// }
    /// ```
    pub fn generate(&self, user_id: u64) -> ResultError<SessionPayload> {
        let key = self.get_combined_key();
        let random_16 = *Uuid::new_v4().as_bytes();
        let now = chrono::Utc::now().timestamp();

        let time_8 = now.to_be_bytes();
        let user_8 = user_id.to_be_bytes();

        let id_binding = user_8.to_hmac_sha256_fixed(&key)?;

        let mut buffer = [0u8; 96];
        buffer[0..16].copy_from_slice(&random_16);
        buffer[16..48].copy_from_slice(&id_binding);
        buffer[48..56].copy_from_slice(&time_8);
        buffer[56..64].copy_from_slice(&user_8);

        let final_sign = buffer[0..64].to_hmac_sha256_fixed(&key)?;
        buffer[64..96].copy_from_slice(&final_sign);
        Ok(SessionPayload {
            token: buffer.to_hex(),
            user_id,
            timestamp: now,
            random_16,
        })
    }

    /// Tokenizes the provided user and returns the token as a `String`.
    ///
    /// # Type Parameters
    /// - `T`: A type that implements `AsRef<dyn UserBase>`. This allows the function to accept
    ///        any type that can provide a reference to an implementor of the `UserBase` trait.
    ///
    /// # Parameters
    /// - `user`: An instance of type `T` representing the user to be tokenized. The `id()` method
    ///           of the `UserBase` trait is used to extract the user's ID.
    ///
    /// # Returns
    /// - `Ok(SessionPayload)`: If the user ID is valid (non-negative), the function generates and returns a
    ///                 token as a `SessionPayload`.
    /// - `Err(Error)`: If the user ID is invalid (negative), an error of type `Error` with a message
    ///                 indicating "Invalid user ID" is returned.
    ///
    /// # Errors
    /// - Returns an `Error::invalid_data` if the user ID is less than `0`.
    ///
    /// # Example
    /// ```rust
    /// let user = SomeUserType { id: 42 }; // Where `SomeUserType` implements `UserBase`
    /// let token = your_instance.tokenize_user(user);
    /// match token {
    ///     Ok(token) => println!("User token: {}", token),
    ///     Err(e) => eprintln!("Failed to tokenize user: {:?}", e),
    /// }
    /// ```
    ///
    /// # Notes
    /// - The `user` parameter is expected to have an `id()` method (provided by the `UserBase` trait)
    ///   to fetch the user ID.
    /// - The user ID is expected to be a non-negative integer. Negative IDs are rejected.
    pub fn generate_with(&self, user: &impl UserBase) -> ResultError<SessionPayload> {
        let user = user.id();
        if user < 0 {
            return Err(Error::invalid_data("Negative user id is not allowed"));
        }
        self.generate(user as u64)
    }

    /// Splits and validates a hexadecimal token, extracting its payload if successful.
    ///
    /// The method performs the following operations:
    /// 1. Validates the length of the input `token_hex`.
    /// 2. Decodes the hexadecimal string into raw bytes.
    /// 3. Verifies the outer HMAC signature to check data integrity.
    /// 4. Parses and extracts the random nonce, timestamp, and user ID from the payload.
    /// 5. Verifies the inner HMAC signature to ensure identity linkage.
    ///
    /// ## Arguments
    ///
    /// * `token_hex` - A string slice containing a hexadecimal token that needs to be split and verified.
    ///
    /// ## Returns
    ///
    /// Returns a `Result` containing:
    /// - `Ok(SessionPayload)` consisting of:
    ///   - `user_id` (u64): The ID of the user extracted from the token.
    ///   - `timestamp` (i64): The session timestamp extracted from the token.
    ///   - `random_16` ([u8; 16]): A randomly generated 16-byte nonce.
    /// - `Err(Error)` in case of:
    ///   - Invalid token length.
    ///   - Errors during hexadecimal decoding.
    ///   - Outer signature verification failures.
    ///   - Errors parsing the payload slices.
    ///   - Inner signature verification failures.
    ///
    /// ## Errors
    ///
    /// Possible errors include:
    /// - `Error::invalid_range`: When the token has an incorrect length.
    /// - `Error::invalid_data`: When hexadecimal decoding fails.
    /// - `Error::invalid_length`: When HMAC initialization fails.
    /// - `Error::overflow`: When slicing the payload encounters issues.
    /// - `Error::permission_denied`: When either outer or inner signature verification fails.
    /// - `Error::other`: For other unexpected errors during HMAC initialization.
    ///
    /// ## Example
    ///
    /// ```rust
    /// let token_hex = "abc123..."; // Example hexadecimal token
    /// let parser = TokenParser::new();
    /// match parser.split(token_hex) {
    ///     Ok(payload) => {
    ///         println!("User ID: {}", payload.user_id);
    ///         println!("Timestamp: {}", payload.timestamp);
    ///         // Use the extracted session payload
    ///     }
    ///     Err(e) => {
    ///         eprintln!("Error processing token: {}", e);
    ///     }
    /// }
    /// ```
    ///
    /// This method ensures strong validation and integrity checks, making it suitable for secure
    /// session handling applications.
    pub fn parse(&self, token_hex: &str) -> ResultError<SessionPayload> {
        if token_hex.len() != Self::LENGTH {
            return Err(Error::invalid_range(
                "Token validation failed: incorrect string length",
            ));
        }

        // Standard library hexadecimal decoding
        let bytes = (0..token_hex.len())
            .step_by(2)
            .map(|i| {
                u8::from_str_radix(&token_hex[i..i + 2], 16)
                    .map_err(|e| Error::invalid_data(format!("Hexadecimal decoding failed: {}", e)))
            })
            .collect::<ResultError<Vec<u8>>>()?;

        let key = self.get_combined_key();
        let payload_part = &bytes[0..64];
        let outer_signature = &bytes[64..96];

        // 1. Verify Outer Signature (Integrity Check)
        let mut mac_outer = HmacSha256::new_from_slice(&key)
            .map_err(|e| Error::invalid_length(format!("HMAC initialization failed: {}", e)))?;
        mac_outer.update(payload_part);

        if mac_outer.verify_slice(outer_signature).is_err() {
            return Err(Error::permission_denied(
                "Token integrity check failed: outer signature mismatch",
            ));
        }

        // 2. Safely Extract Components
        let random_16: [u8; 16] = payload_part[0..16]
            .try_into()
            .map_err(|_| Error::overflow("Internal slicing error: nonce"))?;
        let inner_signature = &payload_part[16..48];
        let time_8: [u8; 8] = payload_part[48..56]
            .try_into()
            .map_err(|_| Error::overflow("Internal slicing error: timestamp"))?;
        let user_8: [u8; 8] = payload_part[56..64]
            .try_into()
            .map_err(|_| Error::overflow("Internal slicing error: user_id"))?;

        let user_id = u64::from_be_bytes(user_8);
        let timestamp = i64::from_be_bytes(time_8);

        // 3. Verify Inner Signature (Identity Binding Check)
        let mut mac_inner = HmacSha256::new_from_slice(&key)
            .map_err(|_| Error::other("HMAC initialization failed"))?;
        mac_inner.update(&user_8);

        if mac_inner.verify_slice(inner_signature).is_err() {
            return Err(Error::permission_denied(
                "Identity binding check failed: inner signature mismatch",
            ));
        }

        Ok(SessionPayload {
            token: token_hex.to_string(),
            user_id,
            timestamp,
            random_16,
        })
    }
}
