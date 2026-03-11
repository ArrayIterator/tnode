use crate::cores::system::error::{Error, ResultError};
use hmac::{Hmac, Mac};
use md5::Md5;
use sha1::Sha1;
use sha2::Digest;
use sha2::{Sha224, Sha256, Sha384, Sha512};

pub type HmacMd5 = Hmac<Md5>;
pub type HmacSha1 = Hmac<Sha1>;
pub type HmacSha256 = Hmac<Sha256>;
pub type HmacSha512 = Hmac<Sha512>;
pub type HmacSha384 = Hmac<Sha384>;
pub type HmacSha224 = Hmac<Sha224>;

pub struct Hash;

#[derive(Debug, Clone, PartialEq)]
pub enum HashAlgo {
    MD5,
    SHA1,
    SHA256,
    SHA512,
    SHA384,
    SHA224,
}

impl Hash {
    pub fn hash_as_bytes<T: AsRef<[u8]>>(algo: HashAlgo, data: T) -> Vec<u8> {
        let data = data.as_ref();
        match algo {
            HashAlgo::MD5 => {
                let mut hash = Md5::new();
                hash.update(data);
                hash.finalize().to_vec()
            }
            HashAlgo::SHA1 => {
                let mut hash = Sha1::new();
                hash.update(data);
                hash.finalize().to_vec()
            }
            HashAlgo::SHA256 => {
                let mut hash = Sha256::new();
                hash.update(data);
                hash.finalize().to_vec()
            }
            HashAlgo::SHA512 => {
                let mut hash = Sha512::new();
                hash.update(data);
                hash.finalize().to_vec()
            }
            HashAlgo::SHA384 => {
                let mut hash = Sha384::new();
                hash.update(data);
                hash.finalize().to_vec()
            }
            HashAlgo::SHA224 => {
                let mut hash = Sha224::new();
                hash.update(data);
                hash.finalize().to_vec()
            }
        }
    }
    pub fn hmac_as_bytes<T: AsRef<[u8]>, K: AsRef<[u8]>>(
        algo: HashAlgo,
        data: T,
        key: K,
    ) -> ResultError<Vec<u8>> {
        let data = data.as_ref();
        let key = key.as_ref();

        let bytes = match algo {
            HashAlgo::MD5 => {
                let mut h = HmacMd5::new_from_slice(key).map_err(Error::invalid_length)?;
                h.update(data);
                h.finalize().into_bytes().to_vec()
            }
            HashAlgo::SHA1 => {
                let mut h = HmacSha1::new_from_slice(key).map_err(Error::invalid_length)?;
                h.update(data);
                h.finalize().into_bytes().to_vec()
            }
            HashAlgo::SHA256 => {
                let mut h = HmacSha256::new_from_slice(key).map_err(Error::invalid_length)?;
                h.update(data);
                h.finalize().into_bytes().to_vec()
            }
            HashAlgo::SHA512 => {
                let mut h = HmacSha512::new_from_slice(key).map_err(Error::invalid_length)?;
                h.update(data);
                h.finalize().into_bytes().to_vec()
            }
            HashAlgo::SHA384 => {
                let mut h = HmacSha384::new_from_slice(key).map_err(Error::invalid_length)?;
                h.update(data);
                h.finalize().into_bytes().to_vec()
            }
            HashAlgo::SHA224 => {
                let mut h = HmacSha224::new_from_slice(key).map_err(Error::invalid_length)?;
                h.update(data);
                h.finalize().into_bytes().to_vec()
            }
        };
        Ok(bytes)
    }
    pub fn hash<T: AsRef<[u8]>>(algo: HashAlgo, data: T) -> String {
        let bytes = Self::hash_as_bytes(algo, data);
        let mut s = String::with_capacity(bytes.len() * 2);
        for b in bytes {
            use std::fmt::Write;
            write!(&mut s, "{:02x}", b).unwrap();
        }
        s
    }

    pub fn hmac<T: AsRef<[u8]>, K: AsRef<[u8]>>(
        algo: HashAlgo,
        data: T,
        key: K,
    ) -> ResultError<String> {
        let bytes = Self::hmac_as_bytes(algo, data, key)?;
        let mut s = String::with_capacity(bytes.len() * 2);
        for b in bytes {
            use std::fmt::Write;
            write!(&mut s, "{:02x}", b).unwrap();
        }
        Ok(s)
    }

    pub fn md5<T: AsRef<[u8]>>(data: T) -> String {
        Self::hash(HashAlgo::MD5, data)
    }
    pub fn sha1<T: AsRef<[u8]>>(data: T) -> String {
        Self::hash(HashAlgo::SHA1, data)
    }
    pub fn sha256<T: AsRef<[u8]>>(data: T) -> String {
        Self::hash(HashAlgo::SHA256, data)
    }
    pub fn sha512<T: AsRef<[u8]>>(data: T) -> String {
        Self::hash(HashAlgo::SHA512, data)
    }
    pub fn sha384<T: AsRef<[u8]>>(data: T) -> String {
        Self::hash(HashAlgo::SHA384, data)
    }
    pub fn sha224<T: AsRef<[u8]>>(data: T) -> String {
        Self::hash(HashAlgo::SHA224, data)
    }
    pub fn hmac_md5<T: AsRef<[u8]>, K: AsRef<[u8]>>(data: T, key: K) -> ResultError<String> {
        Self::hmac(HashAlgo::MD5, data, key)
    }
    pub fn hmac_sha1<T: AsRef<[u8]>, K: AsRef<[u8]>>(data: T, key: K) -> ResultError<String> {
        Self::hmac(HashAlgo::SHA1, data, key)
    }
    pub fn hmac_sha256<T: AsRef<[u8]>, K: AsRef<[u8]>>(data: T, key: K) -> ResultError<String> {
        Self::hmac(HashAlgo::SHA256, data, key)
    }
    pub fn hmac_sha512<T: AsRef<[u8]>, K: AsRef<[u8]>>(data: T, key: K) -> ResultError<String> {
        Self::hmac(HashAlgo::SHA512, data, key)
    }
    pub fn hmac_sha384<T: AsRef<[u8]>, K: AsRef<[u8]>>(data: T, key: K) -> ResultError<String> {
        Self::hmac(HashAlgo::SHA384, data, key)
    }
    pub fn hmac_sha224<T: AsRef<[u8]>, K: AsRef<[u8]>>(data: T, key: K) -> ResultError<String> {
        Self::hmac(HashAlgo::SHA224, data, key)
    }
    #[inline]
    fn ct_eq(a: &[u8], b: &[u8]) -> bool {
        if a.len() != b.len() {
            return false;
        }
        let mut diff: u8 = 0;
        for (&x, &y) in a.iter().zip(b.iter()) {
            diff |= x ^ y;
        }
        diff == 0
    }
}

pub trait HashTrait<T> {
    fn to_sha1(&self) -> T;
    fn to_md5(&self) -> T;
    fn to_sha256(&self) -> T;
    fn to_sha512(&self) -> T;
    fn to_sha384(&self) -> T;
    fn to_sha224(&self) -> T;
    fn to_hmac_sha1<W: AsRef<[u8]>>(&self, key: W) -> ResultError<T>;
    fn to_hmac_md5<W: AsRef<[u8]>>(&self, key: W) -> ResultError<T>;
    fn to_hmac_sha256<W: AsRef<[u8]>>(&self, key: W) -> ResultError<T>;
    fn to_hmac_sha512<W: AsRef<[u8]>>(&self, key: W) -> ResultError<T>;
    fn to_hmac_sha384<W: AsRef<[u8]>>(&self, key: W) -> ResultError<T>;
    fn to_hmac_sha224<W: AsRef<[u8]>>(&self, key: W) -> ResultError<T>;
}

pub trait HashTraitFixed {
    fn to_sha1_fixed(&self) -> [u8; 20];
    fn to_md5_fixed(&self) -> [u8; 16];
    fn to_sha256_fixed(&self) -> [u8; 32];
    fn to_sha512_fixed(&self) -> [u8; 64];
    fn to_sha384_fixed(&self) -> [u8; 48];
    fn to_sha224_fixed(&self) -> [u8; 28];
    fn to_hmac_sha1_fixed<W: AsRef<[u8]>>(&self, key: W) -> ResultError<[u8; 20]>;
    fn to_hmac_sha256_fixed<W: AsRef<[u8]>>(&self, key: W) -> ResultError<[u8; 32]>;
}

pub trait CtEq<Rhs: ?Sized = Self> {
    fn ct_eq(&self, other: &Rhs) -> bool;
}
impl<T, U> CtEq<U> for T
where
    T: AsRef<[u8]> + ?Sized,
    U: AsRef<[u8]> + ?Sized,
{
    fn ct_eq(&self, other: &U) -> bool {
        Hash::ct_eq(self.as_ref(), other.as_ref())
    }
}

impl<S: AsRef<[u8]> + ?Sized> HashTrait<String> for S {
    fn to_sha1(&self) -> String {
        Hash::sha1(self)
    }
    fn to_md5(&self) -> String {
        Hash::md5(self)
    }
    fn to_sha256(&self) -> String {
        Hash::sha256(self)
    }
    fn to_sha512(&self) -> String {
        Hash::sha512(self)
    }
    fn to_sha384(&self) -> String {
        Hash::sha384(self)
    }
    fn to_sha224(&self) -> String {
        Hash::sha224(self)
    }

    fn to_hmac_sha1<W: AsRef<[u8]>>(&self, key: W) -> ResultError<String> {
        Hash::hmac_sha1(self, key)
    }
    fn to_hmac_md5<W: AsRef<[u8]>>(&self, key: W) -> ResultError<String> {
        Hash::hmac_md5(self, key)
    }
    fn to_hmac_sha256<W: AsRef<[u8]>>(&self, key: W) -> ResultError<String> {
        Hash::hmac_sha256(self, key)
    }
    fn to_hmac_sha512<W: AsRef<[u8]>>(&self, key: W) -> ResultError<String> {
        Hash::hmac_sha512(self, key)
    }
    fn to_hmac_sha384<W: AsRef<[u8]>>(&self, key: W) -> ResultError<String> {
        Hash::hmac_sha384(self, key)
    }
    fn to_hmac_sha224<W: AsRef<[u8]>>(&self, key: W) -> ResultError<String> {
        Hash::hmac_sha224(self, key)
    }
}


impl<S: AsRef<[u8]> + ?Sized> HashTraitFixed for S {
    fn to_sha1_fixed(&self) -> [u8; 20] {
        let mut h = Sha1::new();
        h.update(self.as_ref());
        h.finalize().into()
    }

    fn to_md5_fixed(&self) -> [u8; 16] {
        let mut h = Md5::new();
        h.update(self.as_ref());
        h.finalize().into()
    }

    fn to_sha256_fixed(&self) -> [u8; 32] {
        let mut h = Sha256::new();
        h.update(self.as_ref());
        h.finalize().into()
    }

    fn to_sha512_fixed(&self) -> [u8; 64] {
        let mut h = Sha512::new();
        h.update(self.as_ref());
        h.finalize().into()
    }

    fn to_sha384_fixed(&self) -> [u8; 48] {
        let mut h = Sha384::new();
        h.update(self.as_ref());
        h.finalize().into()
    }

    fn to_sha224_fixed(&self) -> [u8; 28] {
        let mut h = Sha224::new();
        h.update(self.as_ref());
        h.finalize().into()
    }

    fn to_hmac_sha1_fixed<W: AsRef<[u8]>>(&self, key: W) -> ResultError<[u8; 20]> {
        let mut h = HmacSha1::new_from_slice(key.as_ref()).map_err(Error::invalid_length)?;
        h.update(self.as_ref());
        Ok(h.finalize().into_bytes().into())
    }

    fn to_hmac_sha256_fixed<W: AsRef<[u8]>>(&self, key: W) -> ResultError<[u8; 32]> {
        let mut h = HmacSha256::new_from_slice(key.as_ref()).map_err(Error::invalid_length)?;
        h.update(self.as_ref());
        Ok(h.finalize().into_bytes().into())
    }
}
