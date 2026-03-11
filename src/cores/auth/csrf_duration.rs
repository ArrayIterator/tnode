use crate::cores::helper::hack::{IsHex, IsNumeric};
use crate::cores::helper::hash::{CtEq, HashTrait};
use crate::cores::system::error::ResultError;
use chrono::Utc;
use std::time::Duration;

const HMAC_HEX_LEN: usize = 64;
const WINDOW_LEN: usize = 12;
const NONCE_LEN: usize = HMAC_HEX_LEN + WINDOW_LEN;
const DEFAULT_DURATION: Duration = Duration::from_secs(60 * 30);

#[derive(Clone, Debug)]
pub struct CsrfDuration {
    key: Vec<u8>,
    duration: Duration,
}

impl CsrfDuration {
    pub fn new<Secret: AsRef<str>, Salt: AsRef<str>>(
        secret: Secret,
        salt: Salt,
        duration: Option<Duration>,
    ) -> Self {
        let mut key = Vec::with_capacity(secret.as_ref().len() + salt.as_ref().len());
        key.extend_from_slice(secret.as_ref().as_bytes());
        key.extend_from_slice(salt.as_ref().as_bytes());

        Self {
            key,
            duration: duration.unwrap_or(DEFAULT_DURATION),
        }
    }

    fn current_window(&self) -> i64 {
        let now = Utc::now().timestamp();
        now / self.duration.as_secs() as i64
    }

    pub fn generate<A: AsRef<str>>(
        &self,
        action: A,
        user_id: u64,
    ) -> ResultError<String> {
        let window = self.current_window();
        let window_str = format!("{:012}", window);

        let mut data = String::with_capacity(32 + action.as_ref().len());
        data.push_str(&user_id.to_string());
        data.push(':');
        data.push_str(action.as_ref());
        data.push(':');
        data.push_str(&window_str);

        let hashed = data.to_hmac_sha256(&self.key)?;

        Ok(format!("{}{}", hashed, window_str))
    }

    pub fn verify<A: AsRef<str>>(
        &self,
        nonce: &str,
        action: A,
        user_id: u64,
    ) -> ResultError<bool> {
        if nonce.len() != NONCE_LEN {
            return Ok(false);
        }

        let (hashed_part, window_part) = nonce.split_at(HMAC_HEX_LEN);

        if !hashed_part.is_hex() || !window_part.is_numeric() {
            return Ok(false);
        }

        let window = match window_part.parse::<i64>() {
            Ok(w) => w,
            Err(_) => return Ok(false),
        };

        let now_window = self.current_window();

        // tolerance ±1 window
        if window < now_window - 1 || window > now_window + 1 {
            return Ok(false);
        }

        let window_str = format!("{:012}", window);

        let mut data = String::with_capacity(32 + action.as_ref().len());
        data.push_str(&user_id.to_string());
        data.push(':');
        data.push_str(action.as_ref());
        data.push(':');
        data.push_str(&window_str);

        let expected = data.to_hmac_sha256(&self.key)?;
        Ok(expected.ct_eq(hashed_part))
    }
}
