use std::collections::HashSet;
use std::fs::File;
use std::io::{BufReader};
use std::net::IpAddr;
use std::path::Path;
use std::sync::{Arc};
use dashmap::DashMap;
use rcgen::{CertificateParams, DistinguishedName, KeyPair, PKCS_ECDSA_P256_SHA256, SanType, SigningKey};
use rustls::crypto::aws_lc_rs::sign::any_ecdsa_type;
use rustls::server::{ClientHello, ParsedCertificate, ResolvesServerCert};
use rustls::{sign, Error};
use rustls::client::verify_server_name;
use rustls::sign::CertifiedKey;
use rustls_pki_types::{CertificateDer, DnsName, PrivateKeyDer, ServerName};
use x509_certificate::{CapturedX509Certificate, X509Certificate};
use crate::cores::idna::domain::Domain;
use crate::factory::config::SSLConfig;

#[derive(Debug, Clone)]
pub struct SSLStorage {
    ssl_config: Arc<SSLConfig>,
    factory_key: Arc<(KeyPair, PrivateKeyDer<'static>)>,
    map: Arc<DashMap<String, Arc<CertifiedKey>>>,
    predefined: Arc<DashMap<String, Arc<CertifiedKey>>>,
    defaults: Arc<DashMap<String, Arc<CertifiedKey>>>,
}

impl ResolvesServerCert for SSLStorage {
    fn resolve(&self, client_hello: ClientHello<'_>) -> Option<Arc<CertifiedKey>> {
        match client_hello.server_name() {
            Some(domain) => self.try_or_create_found_map(domain),
            None => None,
        }
    }
}

impl SSLStorage {
    pub fn new(ssl: SSLConfig) -> Self {
        let object = Self {
            ssl_config: Arc::new(ssl),
            factory_key: Arc::new(Self::generate_key()),
            // claiming 8192 is big enough for most use cases, and avoids resizing overhead
            map: Arc::new(DashMap::with_capacity(8192)),
             // we can store some default certificates here, such as for localhost, etc.
            defaults: Arc::new(DashMap::with_capacity(1024)),
            predefined: Arc::new(DashMap::new()),
        };
        object.init_config_ssl();
        object
    }

    fn inserts_data(&self, key_file: &str, cert_file: &str, domain: Option<String>) {
        if key_file.trim().is_empty() || cert_file.trim().is_empty() {
            return;
        }
        let key_pair = Self::generate_key_pair_from_file(&key_file);
        if key_pair.is_err() {
            return;
        }
        let key_pair = key_pair.unwrap();
        let cert_f = File::open(&cert_file);
        if cert_f.is_err() {
            return;
        }
        let cert_f = cert_f.unwrap();
        let mut certs = Vec::new();
        let mut cert_reader = BufReader::new(cert_f);
        for c in rustls_pemfile::certs(&mut cert_reader) {
            if c.is_err() {
                return;
            }
            certs.push(c.unwrap());
        }
        let cert = certs.iter().find(|e|{
            if let Ok(c) = X509Certificate::from_der(e) {
                // we can check if the certificate is valid by checking if it has a common name or SANs
                if c.subject_common_name().is_some() || c.iter_extensions().any(|ext| ext.id.as_ref() == &[2, 5, 29, 17]) {
                    return true;
                }
            }
            return false;
        });
        if cert.is_none() {
            return;
        }
        let cert = cert.unwrap();
        if !Self::is_key_matched(&key_pair, cert.clone()) {
            return;
        }
        let sans = Self::get_list_sans_from_der(&cert);
        if sans.is_err() {
            return;
        }
        let sans = sans.unwrap();
        if sans.is_empty() {
            return;
        }
        match self.generate_certified_key(certs, &key_pair) {
            Ok(cert) => {
                let cert = Arc::new(cert);
                let domain: &str = if let Some(domain) = domain {
                    &domain.trim().to_ascii_lowercase()
                } else {
                    ""
                };
                if let Ok(ip) = domain.parse::<IpAddr>() {
                    // if domain is an IP address, we can use it directly as SAN
                    let ip_str = format!("{}", ip);
                    self.defaults.insert(ip_str.clone(), cert.clone());
                } else if !domain.is_empty() {
                    let mut domain = domain.to_string();
                    let domain_parsed = Domain::parse_only(&domain);
                    if domain_parsed.is_ok() {
                        let ascii_domain = domain_parsed.unwrap().ascii.to_ascii_lowercase();
                        if ascii_domain.clone() != domain.clone() {
                            domain = ascii_domain;
                        }
                    }
                    // if domain is provided and not an IP, we can use it directly as SAN
                    self.defaults.insert(domain, cert.clone());
                }
                for san in sans {
                    self.defaults.insert(san, cert.clone());
                }
            },
            Err(_) => {},
        }
    }

    // appending into defaults map, so it will be used for matching SNI
    fn init_config_ssl(&self) {
        self.inserts_data(&self.ssl_config.key(), &self.ssl_config.cert(), None);
        if let Some(domains) = self.ssl_config.domains() {
            for (i, k) in domains {
                self.inserts_data(&k.key(), &k.cert(), Some(i.clone()));
            }
        }
    }

    pub fn is_key_matched<C: AsRef<[u8]>>(key: &KeyPair, cert: C) -> bool {
        let cert = cert.as_ref().to_vec();
        // this is a simple check to see if the private key matches the certificate,
        //  by comparing the public key in the certificate with the public key derived from the private key
        let cert = match CapturedX509Certificate::from_der(cert) {
            Ok(c) => c,
            Err(_) => return false,
        };
        let message = b"verify-key-match-123";
        let signature = match key.sign(message) {
            Ok(s) => s,
            Err(_) => return false,
        };
        cert.verify_signed_data(message, signature).is_ok()
    }

    pub fn certificate_der_from_string_file<T: AsRef<Path>>(file: T) -> Result<CertificateDer<'static>, Error> {
        // sizes include ca certs
        let max_cert_size = 50 * 1024; // 50 KB, should be enough for most certificates
        let min_size = 300; // 300 is minimal size for a certificate, to avoid processing invalid files
        let metadata = std::fs::metadata(&file).map_err(|e| Error::Other(rustls::OtherError(Arc::new(e))))?;
        if metadata.len() > max_cert_size {
            return Err(Error::Other(rustls::OtherError(Arc::new(std::io::Error::new(std::io::ErrorKind::InvalidData, "Certificate file is too large")))));
        }
        if metadata.len() < min_size {
            return Err(Error::Other(rustls::OtherError(Arc::new(std::io::Error::new(std::io::ErrorKind::InvalidData, "Certificate file is too small")))));
        }
        let cert_f = File::open(&file).map_err(|e| Error::Other(rustls::OtherError(Arc::new(e))))?;
        let mut cert_reader = BufReader::new(cert_f);
        for c in rustls_pemfile::certs(&mut cert_reader) {
            if c.is_err() {
                return Err(Error::Other(rustls::OtherError(Arc::new(std::io::Error::new(std::io::ErrorKind::InvalidData, "Failed to parse certificate file")))));
            }
            let unwrap = c.unwrap();
            if let Ok(c) = X509Certificate::from_der(unwrap.clone()) {
                // we can check if the certificate is valid by checking if it has a common name or SANs
                if c.subject_common_name().is_some() || c.iter_extensions().any(|ext| ext.id.as_ref() == &[2, 5, 29, 17]) {
                    return Ok(unwrap);
                }
            }
        }
        Err(Error::Other(rustls::OtherError(Arc::new(std::io::Error::new(std::io::ErrorKind::InvalidData, "No valid certificate found in file")))))
    }

    pub fn certificate_der_from_string<T: AsRef<[u8]>>(der: T) -> Result<CertificateDer<'static>, Error> where Self: Sized {
        let der = der.as_ref();
        let mut reader = std::io::BufReader::new(der); // &[u8] otomatis jadi BufRead
        let reader_dyn: &mut dyn std::io::BufRead = &mut reader;
        for c in rustls_pemfile::read_all(reader_dyn) {
            if c.is_err() {
                return Err(Error::Other(rustls::OtherError(Arc::new(std::io::Error::new(std::io::ErrorKind::InvalidData, "Failed to parse certificate file")))));
            }
            let item = c.unwrap();
            match item {
                rustls_pemfile::Item::X509Certificate(certificate_der) => {
                    if let Ok(c) = X509Certificate::from_der(certificate_der.clone()) {
                        // we can check if the certificate is valid by checking if it has a common name or SANs
                        if c.subject_common_name().is_some() || c.iter_extensions().any(|ext| ext.id.as_ref() == &[2, 5, 29, 17]) {
                            return Ok(certificate_der);
                        }
                    }
                },
                _ => {},
            }
        }
        Err(Error::Other(rustls::OtherError(Arc::new(std::io::Error::new(std::io::ErrorKind::InvalidData, "No valid certificate found in file")))))
    }

    pub fn get_list_sans_from_der<T: AsRef<[u8]>>(cert: T) -> Result<Vec<String>, Error> {
        let cert = match X509Certificate::from_der(cert) {
            Ok(c) => c,
            Err(e) => return Err(Error::Other(rustls::OtherError(Arc::new(e)))),
        };
        // OID Subject Alternative Name: 2.5.29.17
        let san_oid = &[2, 5, 29, 17];
        let mut additional = HashSet::new();
        if let Some(cn) = cert.subject_common_name() {
            additional.insert(cn);
        }
        for ext in cert.iter_extensions() {
            if ext.id.as_ref() == san_oid {
                let bytes = ext.value.to_bytes();
                let mut i = 0;

                while i < bytes.len() {
                    let tag = bytes[i];
                    let len = bytes[i + 1] as usize;
                    let start = i + 2;
                    let end = start + len;

                    if end <= bytes.len() {
                        match tag {
                            0x82 => {
                                // DNS Name
                                if let Ok(san_str) = String::from_utf8(bytes[start..end].to_vec()) {
                                    additional.insert(san_str);
                                }
                            }
                            0x87 => {
                                // IP Address
                                let ip_bytes = &bytes[start..end];
                                let ip_str = if ip_bytes.len() == 4 {
                                    // IPv4
                                    format!(
                                        "{}.{}.{}.{}",
                                        ip_bytes[0], ip_bytes[1], ip_bytes[2], ip_bytes[3]
                                    )
                                } else if ip_bytes.len() == 16 {
                                    use std::net::{IpAddr, Ipv6Addr};
                                    let mut arr = [0u8; 16];
                                    arr.copy_from_slice(ip_bytes);
                                    IpAddr::V6(Ipv6Addr::from(arr)).to_string()
                                } else {
                                    "".to_string()
                                };
                                if !ip_str.is_empty() {
                                    additional.insert(ip_str);
                                }
                            }
                            _ => {} // skip
                        }
                    }
                    i = end;
                }
            }
        }
        Ok(additional.iter().map(|s| s.to_string().to_ascii_lowercase()).collect())
    }

    pub fn generate_key() -> (KeyPair, PrivateKeyDer<'static>) {
        // it should not error
        let key_pair = KeyPair::generate_for(&PKCS_ECDSA_P256_SHA256).unwrap();
        let key_der_bytes = key_pair.serialize_der();
        let pkcs_8 = key_der_bytes.into();
        let private_key_der = PrivateKeyDer::Pkcs8(pkcs_8);
        (key_pair, private_key_der)
    }

    pub fn get_factory_key(&self) -> Arc<(KeyPair, PrivateKeyDer<'static>)> {
        self.factory_key.clone()
    }

    fn try_or_create_found_map<T: Into<String>>(&self, domain: T) -> Option<Arc<CertifiedKey>> {
        let domain = domain.into().to_lowercase();
        if let Some(entry) = self.predefined.get(&domain) {
            return Some(entry.value().clone());
        }
        if let Some(entry) = self.defaults.get(&domain) {
            return Some(entry.value().clone());
        }
        if let Some(entry) = self.map.get(&domain) {
            return Some(entry.value().clone());
        }
        let ascii_domain = Domain::idn_to_ascii(&domain).to_ascii_lowercase();
        if ascii_domain != domain {
            if let Some(entry) = self.predefined.get(&ascii_domain) {
                return Some(entry.value().clone());
            }
            if let Some(entry) = self.defaults.get(&ascii_domain) {
                return Some(entry.value().clone());
            }
            if let Some(entry) = self.map.get(&ascii_domain) {
                return Some(entry.value().clone());
            }
        }
        let wildcard_domain = match Domain::parse(&domain) {
            Ok(e) => {
                let main_domain = e.ascii.root;
                let sub_domain = e.ascii.subdomain;
                if let Some(sub_domain) = sub_domain {
                    if let Some((_first, rest)) = sub_domain.split_once('.') {
                        // Jika subdomain adalah "a.b", maka rest adalah "b"
                        // Hasil: "*.b.example.com"
                        format!("*.{}.{}", rest, main_domain)
                    } else {
                        // Jika subdomain cuma satu level ("b") atau kosong
                        // Hasil: "*.example.com"
                        format!("*.{}", main_domain)
                    }
                } else {
                    // Jika tidak ada subdomain, hasilnya tetap "*.example.com"
                    format!("*.{}", main_domain)
                }
            },
            Err(_) => {
                match Domain::parse_only(&domain) {
                    Ok(d) => d.ascii,
                    Err(_) => ascii_domain,
                }
            }
        };
        if let Some(entry) = self.predefined.get(&wildcard_domain) {
            return Some(entry.value().clone());
        }
        if let Some(entry) = self.defaults.get(&wildcard_domain) {
            return Some(entry.value().clone());
        }
        if let Some(entry) = self.map.get(&wildcard_domain) {
            return Some(entry.value().clone());
        }
        if ! self.ssl_config.auto_create() {
            return None;
        }
        match self.generate_wildcard_certificate(&wildcard_domain) {
            Ok(certified_key) => {
                let certified_key = Arc::new(certified_key);
                if self.map.len() >= 8192 {
                    // reduce 100 from start
                    let keys_to_remove: Vec<String> = self.map.iter().take(100).map(|entry| entry.key().clone()).collect();
                    for key in keys_to_remove {
                        self.map.remove(&key);
                    }
                }
                self.map.insert(wildcard_domain, certified_key.clone());
                Some(certified_key)
            },
            Err(_) => None,
        }
    }

    pub fn generate_key_pair_from_file<T: AsRef<Path>>(file: T) -> Result<KeyPair, Error> where Self: Sized {
        // get size
        let max_size_key = 10 * 1024; // 10 KB, should be enough for most private keys
        let min_size = 300; // 300 is minimal size for a private key, to avoid processing invalid files
        let metadata = std::fs::metadata(&file).map_err(|e| Error::Other(rustls::OtherError(Arc::new(e))))?;
        if metadata.len() < min_size {
            return Err(Error::Other(rustls::OtherError(Arc::new(std::io::Error::new(std::io::ErrorKind::InvalidData, "Key file is too small")))));
        }
        if metadata.len() > max_size_key {
            return Err(Error::Other(rustls::OtherError(Arc::new(std::io::Error::new(std::io::ErrorKind::InvalidData, "Key file is too large")))));
        }
        let content = std::fs::read_to_string(file).map_err(|e| Error::Other(rustls::OtherError(Arc::new(e))))?;
        Self::generate_key_pair_from_string(content)
    }

    pub fn generate_key_pair_from_string<T: Into<String>>(content: T) -> Result<KeyPair, Error> where Self: Sized {
        let content = content.into();
        let key_pair = KeyPair::from_pem(content.as_str()).map_err(|e| Error::Other(rustls::OtherError(Arc::new(e))))?;
        Ok(key_pair)
    }

    pub fn generate_wildcard_certificate<T: AsRef<str>>(&self, domain: T) -> Result<CertifiedKey, Error> where Self: Sized {
        let (key_pair, _) = &*self.get_factory_key();
        Ok(self.generate_wildcard_certificate_with_keypair(domain, key_pair)?)
    }

    pub fn generate_certified_key(&self, cert: Vec<CertificateDer<'static>>, key_pair: &KeyPair) -> Result<CertifiedKey, Error> where Self: Sized {
        let key_der_bytes = key_pair.serialize_der();
        let private_key_der = PrivateKeyDer::Pkcs8(key_der_bytes.into());
        let signing_key = any_ecdsa_type(&private_key_der)?;
        let certified_key = CertifiedKey::new(cert.clone(), signing_key);
        Ok(certified_key)
    }

    pub fn generate_wildcard_certificate_with_keypair<T: AsRef<str>>(&self, domain: T, key_pair: &KeyPair) -> Result<CertifiedKey, Error> where Self: Sized {
        let mut domain = domain.as_ref();
        let mut params = CertificateParams::default();
        params.distinguished_name = DistinguishedName::new();
        if let Ok(ip) = domain.parse::<IpAddr>() {
            params.distinguished_name.push(rcgen::DnType::CommonName, format!("{}", &domain));
            params.subject_alt_names = vec![
                SanType::IpAddress(ip),
            ];
        } else {
            if domain.starts_with("*.") {
                domain = &domain[2..];
            }
            let domain_name = Domain::parse_only(domain).map_err(|e|Error::Other(rustls::OtherError(Arc::new(e))))?.ascii;
            // todo: we can cache the generated certificate for the same domain, but it is not a common case, so we can ignore it for now
            params.distinguished_name.push(rcgen::DnType::CommonName, format!("{}", &domain_name));
            params.subject_alt_names = vec![
                SanType::DnsName(domain_name.clone().try_into().unwrap()),
                SanType::DnsName(format!("*.{}", &domain_name).try_into().unwrap()),
            ];
        }
        params.is_ca = rcgen::IsCa::NoCa;
        let cert = params.self_signed(&key_pair).map_err(|e| Error::Other(rustls::OtherError(Arc::new(e))))?;
        Ok(self.generate_certified_key(vec![cert.der().clone()], key_pair)?)
    }

    /// Add a new `sign::CertifiedKey` to be used for the given SNI `name`.
    ///
    /// This function fails if `name` is not a valid DNS name, or if
    /// it's not valid for the supplied certificate, or if the certificate
    /// chain is syntactically faulty.
    pub fn add(&self, name: &str, ck: sign::CertifiedKey) -> Result<(), Error> {
        let server_name = {
            let checked_name = DnsName::try_from(name)
                .map_err(|_| Error::General("Bad DNS name".into()))
                .map(|name| name.to_lowercase_owned())?;
            ServerName::DnsName(checked_name)
        };

        // Check the certificate chain for validity:
        // - it should be non-empty list
        // - the first certificate should be parsable as a x509v3,
        // - the first certificate should quote the given server name
        //   (if provided)
        //
        // These checks are not security-sensitive.  They are the
        // *server* attempting to detect accidental misconfiguration.

        ck.end_entity_cert()
            .and_then(ParsedCertificate::try_from)
            .and_then(|cert| verify_server_name(&cert, &server_name))?;

        if let ServerName::DnsName(name) = server_name {
            self.predefined.insert(name.as_ref().to_string(), Arc::new(ck));
        }
        Ok(())
    }

    pub fn remove_predefined(&self, name: &str) {
        let name = name.to_lowercase();
        self.predefined.remove(&name);
    }
}
