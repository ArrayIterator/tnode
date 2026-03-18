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
    pub(crate) timestamp: u64,
    /// The UNIX timestamp representing the token expiration time (derived from `timestamp` + duration).
    pub(crate) expired_at: u64,
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
    pub fn timestamp(&self) -> u64 {
        self.timestamp
    }
    pub fn expired_at(&self) -> u64 {
        self.expired_at
    }
    pub fn random(&self) -> &[u8; 16] {
        &self.random_16
    }
    pub fn is_expired(&self) -> bool {
        let now = chrono::Utc::now().timestamp() as u64;
        let timestamp = self.timestamp;
        let expired = self.expired_at;
        return timestamp <= 0 || expired <= 0 || timestamp >= now || expired <= now;
    }
    pub fn is_need_renew(&self, renew_duration: Duration) -> bool {
        let now = chrono::Utc::now().timestamp() as u64;
        let expired = self.expired_at;
        expired > now && expired - now <= renew_duration.as_secs()
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
    pub const LENGTH: usize = 208;

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

    fn get_combined_key(&self) -> Vec<u8> {
        [self.secret.as_slice(), self.salt.as_slice()].concat()
    }

    pub fn generate(&self, user_id: u64, duration: Duration) -> ResultError<SessionPayload> {
        let key = self.get_combined_key();
        let random_16 = *Uuid::new_v4().as_bytes();
        let now = chrono::Utc::now().timestamp() as u64;
        let expired_at = now + duration.as_secs();

        let time_8 = now.to_be_bytes();
        let exp_8 = expired_at.to_be_bytes(); // Convert expired_at ke bytes
        let user_8 = user_id.to_be_bytes();

        let id_binding = user_8.to_hmac_sha256_fixed(&key)?;

        // Buffer naik jadi 104 bytes untuk menampung exp_8
        let mut buffer = [0u8; 104];
        buffer[0..16].copy_from_slice(&random_16);
        buffer[16..48].copy_from_slice(&id_binding);
        buffer[48..56].copy_from_slice(&time_8);
        buffer[56..64].copy_from_slice(&exp_8);      // Masukkan expired_at
        buffer[64..72].copy_from_slice(&user_8);

        // Final sign sekarang mencakup bytes 0 sampai 72
        let final_sign = buffer[0..72].to_hmac_sha256_fixed(&key)?;
        buffer[72..104].copy_from_slice(&final_sign);

        Ok(SessionPayload {
            token: buffer.to_hex(),
            user_id,
            timestamp: now,
            random_16,
            expired_at: expired_at,
        })
    }

    pub fn generate_unchecked(&self, user_id: u64, duration: Duration) -> SessionPayload {
        self.generate(user_id, duration).unwrap()
    }

    pub fn generate_with(&self, user: &impl UserBase, duration: Duration) -> ResultError<SessionPayload> {
        let user = user.id();
        if user < 0 {
            return Err(Error::invalid_data("Negative user id is not allowed"));
        }
        let user = user as u64;
        self.generate(user, duration)
    }

    pub fn parse(&self, token_hex: &str) -> ResultError<SessionPayload> {
        if token_hex.len() != Self::LENGTH {
            return Err(Error::invalid_range("Token validation failed: incorrect string length"));
        }

        // Hex decoding (tetap sama)
        let bytes = (0..token_hex.len())
            .step_by(2)
            .map(|i| {
                u8::from_str_radix(&token_hex[i..i + 2], 16)
                    .map_err(|e| Error::invalid_data(format!("Hexadecimal decoding failed: {}", e)))
            })
            .collect::<ResultError<Vec<u8>>>()?;

        let key = self.get_combined_key();
        // Payload sekarang 72 bytes, Signature 32 bytes terakhir
        let payload_part = &bytes[0..72];
        let outer_signature = &bytes[72..104];

        // 1. Verify Outer Signature
        let mut mac_outer = HmacSha256::new_from_slice(&key)
            .map_err(|e| Error::invalid_length(format!("HMAC initialization failed: {}", e)))?;
        mac_outer.update(payload_part);

        if mac_outer.verify_slice(outer_signature).is_err() {
            return Err(Error::permission_denied("Token integrity check failed: outer signature mismatch"));
        }

        // 2. Extract Components
        let random_16: [u8; 16] = payload_part[0..16].try_into().unwrap();
        let inner_signature = &payload_part[16..48];
        let timestamp = u64::from_be_bytes(payload_part[48..56].try_into().unwrap());
        let expired_at = u64::from_be_bytes(payload_part[56..64].try_into().unwrap());
        let user_8: [u8; 8] = payload_part[64..72].try_into().unwrap();
        let user_id = u64::from_be_bytes(user_8);

        // 3. Verify Inner Signature
        let mut mac_inner = HmacSha256::new_from_slice(&key).map_err(|_| Error::other("HMAC Error"))?;
        mac_inner.update(&user_8);
        if mac_inner.verify_slice(inner_signature).is_err() {
            return Err(Error::permission_denied("Identity binding check failed"));
        }

        Ok(SessionPayload {
            token: token_hex.to_string(),
            user_id,
            timestamp,
            expired_at,
            random_16,
        })
    }
}
