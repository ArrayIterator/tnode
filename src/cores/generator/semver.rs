use crate::cores::system::error::{Error, ResultError};
use std::ops::{Deref, DerefMut};

#[derive(Debug, Clone)]
pub struct Semver {
    semver: semver::Version,
}

impl Semver {
    pub fn new(major: u64, minor: u64, patch: u64) -> Self {
        Self {
            semver: semver::Version::new(major, minor, patch),
        }
    }
    pub fn parse<T: AsRef<str>>(version: T) -> ResultError<semver::Version> {
        let version = version.as_ref();
        semver::Version::parse(version.as_ref()).map_err(Error::parse_error)
    }
}

impl From<semver::Version> for Semver {
    fn from(semver: semver::Version) -> Self {
        Self { semver }
    }
}

impl Deref for Semver {
    type Target = semver::Version;
    fn deref(&self) -> &Self::Target {
        &self.semver
    }
}
impl DerefMut for Semver {
    fn deref_mut(&mut self) -> &mut Self::Target {
        &mut self.semver
    }
}
impl PartialEq for Semver {
    fn eq(&self, other: &Self) -> bool {
        self.semver == other.semver
    }
}
