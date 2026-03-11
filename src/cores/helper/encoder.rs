use crate::cores::system::error::{Error, ResultError};
use base64::prelude::BASE64_STANDARD;
use base64::Engine;

pub struct Encoder;

impl Encoder {
    pub fn base64_encode<T: AsRef<[u8]>>(data: T) -> String {
        BASE64_STANDARD.encode(data)
    }
    pub fn base64_decode<T: AsRef<[u8]>>(data: T) -> ResultError<Vec<u8>> {
        BASE64_STANDARD
            .decode(data)
            .map_err(|e| Error::parse_error(e))
    }
    pub fn base32_encode<T: AsRef<[u8]>>(data: T) -> String {
        base32::encode(base32::Alphabet::Rfc4648 { padding: true }, data.as_ref())
    }
    pub fn base32_decode<T: AsRef<str>>(data: T) -> ResultError<Vec<u8>> {
        let data = data.as_ref();
        base32::decode(base32::Alphabet::Rfc4648 { padding: true }, data)
            .ok_or(Error::encoding("Invalid base32 string".to_string()))
    }
    pub fn url_encode<T: AsRef<str>>(data: T) -> String {
        urlencoding::encode(data.as_ref()).into_owned()
    }
    pub fn url_encode_binary<T: AsRef<[u8]>>(data: T) -> String {
        urlencoding::encode_binary(data.as_ref()).into_owned()
    }
    pub fn url_decode<T: AsRef<str>>(data: T) -> ResultError<String> {
        Ok(urlencoding::decode(data.as_ref())
            .map_err(|e| Error::parse_error(e))?
            .to_string())
    }
    pub fn url_decode_binary<T: AsRef<[u8]>>(data: T) -> ResultError<Vec<u8>> {
        Ok(urlencoding::decode_binary(data.as_ref()).to_vec())
    }
}
