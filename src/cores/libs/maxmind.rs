use std::fmt::Debug;
use std::sync::{Arc};

use crate::cores::downloader::download_manager::DownloadManager;
use crate::cores::system::error::{Error, ResultError};
// todo: completing

// https://download.maxmind.com/geoip/databases/GeoLite2-ASN/download?suffix=tar.gz
// https://download.maxmind.com/geoip/databases/GeoLite2-City/download?suffix=tar.gz
// https://download.maxmind.com/geoip/databases/GeoLite2-Country/download?suffix=tar.gz

#[derive(Debug, Clone)]
pub enum GeoEditionType {
    GeoLite2ASN(Arc<GeoEdition>, String),
    GeoLite2Country(Arc<GeoEdition>, String),
    GeoLite2City(Arc<GeoEdition>, String),
    GeoIP2ASN(Arc<GeoEdition>, String),
    GeoIP2Country(Arc<GeoEdition>, String),
    GeoIP2City(Arc<GeoEdition>, String),
}

impl GeoEditionType {
    pub fn edition(&self) -> &GeoEdition {
        match self {
            GeoEditionType::GeoLite2ASN(e, _) |
            GeoEditionType::GeoLite2Country(e, _) |
            GeoEditionType::GeoLite2City(e, _) |
            GeoEditionType::GeoIP2ASN(e, _) |
            GeoEditionType::GeoIP2Country(e, _) |
            GeoEditionType::GeoIP2City(e, _) => &e,
        }
    }

    pub fn license_key(&self) -> String {
        match self {
            GeoEditionType::GeoLite2ASN(_, l) |
            GeoEditionType::GeoLite2Country(_, l) |
            GeoEditionType::GeoLite2City(_, l) |
            GeoEditionType::GeoIP2ASN(_, l) |
            GeoEditionType::GeoIP2Country(_, l) |
            GeoEditionType::GeoIP2City(_, l) => l.clone(),
        }
    }

    pub fn identity(&self) -> String {
        match self {
            GeoEditionType::GeoLite2ASN(_, _) => "GeoLite2-ASN",
            GeoEditionType::GeoLite2Country(_, _) => "GeoLite2-Country",
            GeoEditionType::GeoLite2City(_, _) => "GeoLite2-City",
            GeoEditionType::GeoIP2ASN(_, _) => "GeoIP2-ASN",
            GeoEditionType::GeoIP2Country(_, _) => "GeoIP2-Country",
            GeoEditionType::GeoIP2City(_, _) => "GeoIP2-City",
        }.to_string()
    }

    pub fn file_name(&self) -> String {
        format!("{}.mmdb", self.identity())
    }

    fn license_for<L: AsRef<str>, T: AsRef<str>>(&self, license_key: L, license_type: T) -> String {
        format!(
            "https://download.maxmind.com/app/geoip_download?edition_id={}&license_key={}&suffix=tar.gz",
            license_type.as_ref(),
            license_key.as_ref()
        )
    }

    pub fn to_download_url(&self) -> String {
        self.license_for(&self.license_key(), self.identity())
    }
}

#[derive(Debug, Clone)]
pub enum GeoEdition {
    GeoLite2,
    GeoIP2,
}

impl GeoEdition {
    pub fn edition_type_arc<T: Into<MaxMindLicense>>(self: &Arc<Self>, license: T) -> GeoEditionTypes {
        GeoEditionTypes::new(self.clone(), license)
    }
    pub fn edition_type_ref<T: Into<MaxMindLicense>>(&self, license: T) -> GeoEditionTypes {
        GeoEditionTypes::new(Arc::new(self.clone()), license)
    }
    pub fn edition_type<T: Into<MaxMindLicense>>(self, license: T) -> GeoEditionTypes{
        GeoEditionTypes::new(Arc::new(self), license)
    }
}

#[derive(Debug, Clone)]
pub struct GeoEditionTypes {
    license: Arc<MaxMindLicense>,
    edition: Arc<GeoEdition>,
    asn: Arc<GeoEditionType>,
    country: Arc<GeoEditionType>,
    city: Arc<GeoEditionType>,
}

impl GeoEditionTypes {
    pub fn new<T: Into<MaxMindLicense>>(edition: Arc<GeoEdition>, license: T) -> Self {
        let license_key = license.into();
        let edition_types = edition.edition_type_arc(&license_key);
        Self {
            license: Arc::new(license_key),
            edition: edition.clone(),
            asn: Arc::new(edition_types.get_asn().clone()),
            country: Arc::new(edition_types.get_country().clone()),
            city: Arc::new(edition_types.get_city().clone()),
        }
    }

    pub fn with_license<T: Into<MaxMindLicense>>(&self, license: T) -> Self {
        Self::new(self.edition.clone(), license)
    }

    pub fn get_license(&self) -> &MaxMindLicense {
        &self.license
    }
    pub fn edition(&self) -> &GeoEdition {
        &self.edition
    }
    pub fn is_lite(&self) -> bool {
        matches!(self.edition.as_ref(), GeoEdition::GeoLite2)
    }
    pub fn get_asn(&self) -> &GeoEditionType {
        &self.asn
    }
    pub fn get_country(&self) -> &GeoEditionType {
        &self.country
    }
    pub fn get_city(&self) -> &GeoEditionType {
        &self.city
    }
}


#[derive(Debug, Clone)]
pub struct MaxMindLicense {
    license_key: String,
    license_valid: bool,
}

impl MaxMindLicense {
    pub fn new<T: Into<String>>(license: T) -> Self {
        let license_key = license.into();
        Self {
            license_key: license_key.clone(),
            license_valid: Self::validate_license_key(license_key)
        }
    }
    pub fn license_match<T: Into<MaxMindLicense>>(&self, license: T) -> bool {
        self.license_key == license.into().license_key
    }
    pub fn is_valid(&self) -> bool {
        self.license_valid
    }
    pub fn validate_license_key<T: Into<String>>(license: T) -> bool {
        let license = license.into();
        // license length must be 40 chars
        if license.len() != 40 {
            return false;
        }
        let split_underscore = license.split('_').collect::<Vec<&str>>();
        // xxxxxx_xxxxxxxxxxxxxxxxxxxxxxxxxxxxx_mmk & prefix should 6 chars, middle should 3 chars, suffix should be "mmk"
        if split_underscore.len() != 3 {
            return false;
        }
        let prefix = split_underscore[0];
        let middle = split_underscore[1];
        let suffix = split_underscore[2];
        if prefix.len() != 6 || suffix != "mmk" {
            return false;
        }
        prefix.chars().all(|c| c.is_ascii_alphanumeric())
            && middle.chars().all(|c| c.is_ascii_alphanumeric())
    }
    pub fn license_key(&self) -> &str {
        &self.license_key
    }
}

impl From<&str> for MaxMindLicense {
    fn from(license: &str) -> Self {
        Self::new(license)
    }
}

impl From<String> for MaxMindLicense {
    fn from(license: String) -> Self {
        Self::new(&license)
    }
}
impl From<&MaxMindLicense> for MaxMindLicense {
    fn from(license: &MaxMindLicense) -> Self {
        license.clone()
    }
}

impl From<Arc<MaxMindLicense>> for MaxMindLicense {
    fn from(license: Arc<MaxMindLicense>) -> Self {
        Self::from(&license)
    }
}

impl From<&Arc<MaxMindLicense>> for MaxMindLicense {
    fn from(license: &Arc<MaxMindLicense>) -> Self {
        license.as_ref().clone()
    }
}


pub trait GeoIP: Debug + 'static {
    fn get_license(&self) -> &MaxMindLicense;
    fn get_edition(&self) -> &GeoEdition;
}


// todo: Implement download and update logic, and also implement caching logic for the downloaded databases, and also implement the logic to load the databases into memory and query them for IP lookups.
#[derive(Debug, Clone)]
pub struct GeoIPDownload {
    pub asn: String,
    pub country: String,
    pub city: String,
}

#[derive(Debug, Clone)]
pub struct Maxmind {
    pub edition_types: Arc<GeoEditionTypes>,
    download_manager: Arc<DownloadManager>,
}

impl GeoIP for Maxmind {
    fn get_license(&self) -> &MaxMindLicense {
        &self.edition_types.get_license()
    }
    fn get_edition(&self) -> &GeoEdition {
        &self.edition_types.edition()
    }
}

impl Maxmind {
    pub fn new<T: Into<MaxMindLicense>>(
        license: T,
        edition: GeoEdition
    ) -> Self where Self: Sized {
        Self {
            edition_types: Arc::new(edition.edition_type(&license.into())),
            download_manager: Arc::new(DownloadManager::default())
        }
    }

    pub fn set_license_key<T: Into<MaxMindLicense>>(&mut self, license: T) -> ResultError<Arc<GeoEditionTypes>> {
        let license = license.into();
        if !license.is_valid() {
            return Err(Error::invalid_input("Invalid license key"));
        }
        if !self.get_license().license_match(&license) {
            let old_edition_types = self.edition_types.clone();
            self.edition_types = Arc::new(self.edition_types.with_license(&license));
            return Ok(old_edition_types);
        }
        Ok(self.edition_types.clone())
    }

    pub fn get_edition_types(&self) -> &GeoEditionTypes {
        &self.edition_types
    }
}
