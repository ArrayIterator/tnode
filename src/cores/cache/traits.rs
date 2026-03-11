use crate::cores::cache::error::ResultCacheError;
use crate::cores::helper::hack::IsNumeric;
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use std::any::type_name_of_val;
use std::collections::HashMap;
use std::fmt::Debug;

#[derive(Debug, Serialize, Deserialize)]
pub struct CacheData {
    pub key: String,
    pub expiration: u64,
    pub value: CacheValue,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum CacheValue {
    Text(String),
    Number(usize),
    Boolean(bool),
    Vector(Vec<CacheValue>),
    Bytes(Vec<u8>),
    None,
    Object(serde_json::Value),
}

impl Default for CacheValue {
    fn default() -> Self {
        Self::None
    }
}

impl CacheValue {
    pub fn is_text(&self) -> bool {
        matches!(self, Self::Text(_))
    }
    pub fn is_number(&self) -> bool {
        matches!(self, Self::Number(_))
    }
    pub fn is_boolean(&self) -> bool {
        matches!(self, Self::Boolean(_))
    }
    pub fn is_vector(&self) -> bool {
        matches!(self, Self::Vector(_))
    }
    pub fn is_bytes(&self) -> bool {
        matches!(self, Self::Bytes(_))
    }
    pub fn is_object(&self) -> bool {
        matches!(self, Self::Object(_))
    }
    pub fn to_text(&self) -> ResultCacheError<String> {
        match self {
            Self::Text(s) => Ok(s.to_string()),
            Self::None => Ok("".to_string()),
            Self::Number(n) => Ok(n.to_string()),
            Self::Boolean(b) => Ok(b.to_string()),
            Self::Vector(v) => {
                let mut s = String::new();
                for e in v {
                    s.push_str(&e.to_text()?);
                }
                Ok(s)
            }
            _ => Err(crate::cores::cache::error::CacheError::conversion_error(
                "not text".to_string(),
            )),
        }
    }
    pub fn to_number(&self) -> ResultCacheError<usize> {
        match self {
            Self::Number(n) => Ok(*n),
            Self::Boolean(n) => Ok(if *n { 1 } else { 0 }),
            Self::None => Ok(0),
            Self::Bytes(e) => {
                let number = String::from_utf8(e.to_vec()).map_err(|e| {
                    crate::cores::cache::error::CacheError::invalid_value(e.to_string())
                })?;
                if number.is_numeric() {
                    return Ok(number.parse::<usize>().map_err(|e| {
                        crate::cores::cache::error::CacheError::invalid_value(e.to_string())
                    })?);
                }
                Err(crate::cores::cache::error::CacheError::invalid_value(
                    "CacheValue is not a number".to_string(),
                ))
            }
            Self::Text(e) => {
                if e.is_numeric() {
                    return Ok(e.parse::<usize>().map_err(|e| {
                        crate::cores::cache::error::CacheError::conversion_error(e.to_string())
                    })?);
                }
                Err(crate::cores::cache::error::CacheError::invalid_value(
                    "CacheValue is not a number".to_string(),
                ))
            }
            _ => Err(crate::cores::cache::error::CacheError::conversion_error(
                "CacheValue is not a number".to_string(),
            )),
        }
    }
    pub fn to_boolean(&self) -> ResultCacheError<bool> {
        match self {
            Self::Boolean(b) => Ok(*b),
            Self::None => Ok(false),
            Self::Text(s) => Ok(s.parse::<bool>().unwrap_or(false)),
            Self::Number(n) => Ok(*n == 1),
            Self::Vector(v) => Ok(!v.is_empty()),
            Self::Bytes(b) => Ok(!b.is_empty()),
            e => Err(crate::cores::cache::error::CacheError::conversion_error(
                format!("CacheValue not boolean: {:?}", type_name_of_val(e)),
            )),
        }
    }
    pub fn text<S: Into<String>>(s: S) -> Self {
        Self::Text(s.into())
    }
    pub fn number(n: usize) -> Self {
        Self::Number(n)
    }
    pub fn boolean(b: bool) -> Self {
        Self::Boolean(b)
    }
    pub fn vector(v: Vec<Self>) -> Self {
        Self::Vector(v)
    }
    pub fn bytes(b: Vec<u8>) -> Self {
        Self::Bytes(b)
    }
    pub fn object<T: Serialize>(obj: T) -> ResultCacheError<Self> {
        let json = serde_json::to_value(&obj)
            .map_err(|e| crate::cores::cache::error::CacheError::conversion_error(e.to_string()))?;
        Ok(Self::Object(json))
    }
    pub fn to_struct<T: for<'a> Deserialize<'a>>(&self) -> ResultCacheError<T> {
        match self {
            Self::Object(v) => Ok(serde_json::from_value::<T>(v.clone()).map_err(|e| {
                crate::cores::cache::error::CacheError::conversion_error(e.to_string())
            })?),
            _ => Err(crate::cores::cache::error::CacheError::conversion_error(
                format!(
                    "CacheValue not struct: {:?}",
                    type_name_of_val(self)
                ),
            )),
        }
    }
}

pub trait CacheItemTrait: Debug + Send + Sync {
    fn get_key(&self) -> &str;
    fn get(&self) -> ResultCacheError<CacheValue>;
    fn set(&mut self, value: CacheValue) -> ResultCacheError<()>;
    fn get_expiration(&self) -> Option<DateTime<Utc>>;
    fn set_expiration(&mut self, expiration: Option<DateTime<Utc>>);
    fn is_hit(&self) -> bool;
    fn exists(&self) -> bool;
    fn is_regenerating(&self) -> bool;
    fn is_expired(&self) -> bool {
        if let Some(exp) = self.get_expiration() {
            return Utc::now() > exp;
        }
        false
    }
}

pub trait CacheItemPoolTrait: Debug + Send + Sync {
    fn get_item(&self, key: &str) -> ResultCacheError<Box<dyn CacheItemTrait>>;
    fn get_items(&self, keys: Vec<&str>) -> HashMap<String, Box<dyn CacheItemTrait>>;
    fn save_item(&self, item: &dyn CacheItemTrait) -> ResultCacheError<bool>;
    fn save_deferred(&self, item: &dyn CacheItemTrait) -> bool;
    fn clear(&self) -> ResultCacheError<bool>;
    fn delete_items(&self, keys: Vec<&str>) -> ResultCacheError<bool>;
    fn delete_item(&self, key: &str) -> ResultCacheError<bool>;
    fn commit(&self) -> ResultCacheError<bool>;
}
