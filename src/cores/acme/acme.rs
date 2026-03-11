use crate::cores::acme::provider::{
    AcmeProvider, AcmeProviderType, BuyPass, CustomProvider, Google, KeyPairConversion,
    LetsEncrypt, ZeroSSL,
};
use crate::cores::idna::domain::Domain;
use crate::cores::net::ip::Ip;
use crate::cores::system::error::{Error, ResultError};
use chrono::{Duration, Utc};
use instant_acme::{
    Account, AccountCredentials, AuthorizationHandle, AuthorizationStatus, ChallengeHandle,
    ChallengeStatus, Identifier, NewOrder, OrderStatus, RetryPolicy,
};
use log::warn;
use rcgen::{CertificateParams, CertificateSigningRequest, KeyPair};
use rustls_pemfile::certs;
use std::collections::HashMap;
use std::fmt::{Debug, Display, Formatter};
use std::io::Cursor;
use std::net::IpAddr;
use std::ops::Deref;
use std::sync::{Arc, OnceLock};
use x509_certificate::X509Certificate;

pub const RENEW_EXPIRED_DAYS: usize = 30;
pub const MIN_RENEW_EXPIRED_DAYS: usize = 7; // - 7days
pub const MAX_RENEW_EXPIRED_DAYS: usize = 90; // - lets encrypt is 90 days

#[async_trait::async_trait]
pub trait AuthorizationHandler: Debug + Send + Sync + 'static {
    async fn handle(
        &self,
        authorization_handle: &mut AuthorizationHandle,
    ) -> ResultError<ChallengeHandle>;
}

pub struct Acme {
    account: Arc<Account>,
    credentials: Arc<AccountCredentials>,
    provider_type: Arc<AcmeProviderType>,
    handler: Arc<dyn AuthorizationHandler>,
    expired_days: usize,
    staging: bool,
}

#[derive(Debug)]
pub enum AcmeInstance {
    LetsEncrypt(String),
    Google(String),
    ZeroSSL(String),
    BuyPass(String),
    Custom(CustomProvider, String),
}

#[derive(Debug, Clone)]
pub enum RequestType {
    Domain(String),
    Ip(String),
}

impl RequestType {
    pub fn is_valid(&self) -> bool {
        match self {
            RequestType::Domain(e) => {
                if let Ok(e) = Domain::parse(&e) {
                    // use icann
                    return e.icann;
                }
                false
            }
            RequestType::Ip(e) => Ip::public_ip_version(&e).is_some(),
        }
    }
    pub fn to_string(&self) -> String {
        match self {
            RequestType::Domain(e) => e.clone().to_lowercase(),
            RequestType::Ip(e) => e.clone().to_lowercase(),
        }
    }
    pub fn to_identifier_data(&self) -> ResultError<(String, Identifier)> {
        if !self.is_valid() {
            return Err(Error::invalid_input(format!(
                "Requested {} is not valid",
                self
            )));
        }
        Ok(match self {
            RequestType::Domain(e) => {
                let domain = Domain::parse(e)?.ascii.domain;
                (domain.clone(), Identifier::Dns(domain))
            }
            RequestType::Ip(e) => {
                let ip = e.parse::<IpAddr>().map_err(Error::parse_error)?;
                (ip.to_string(), Identifier::Ip(ip))
            }
        })
    }
}

impl Display for RequestType {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        match self {
            RequestType::Domain(e) => {
                write!(f, "Domain({})", self.to_string())
            }
            RequestType::Ip(e) => {
                write!(f, "Ip({})", self.to_string())
            }
        }
    }
}
impl AcmeInstance {
    pub async fn to_acme_instance<T: AsRef<str>>(
        &self,
        key: T,
        staging: bool,
        handler: Arc<impl AuthorizationHandler>,
    ) -> ResultError<Acme> {
        let key = key.as_ref();
        let (provider, (account, credentials)) = match self {
            Self::LetsEncrypt(e) => (
                Arc::new(AcmeProviderType::LetsEncrypt),
                LetsEncrypt::existing_account_info_key(key.to_string(), staging).await?,
            ),
            Self::Google(e) => (
                Arc::new(AcmeProviderType::Google),
                Google::existing_account_info_key(key.to_string(), staging).await?,
            ),
            Self::ZeroSSL(e) => (
                Arc::new(AcmeProviderType::ZeroSSL),
                ZeroSSL::existing_account_info_key(key.to_string(), staging).await?,
            ),
            Self::BuyPass(e) => (
                Arc::new(AcmeProviderType::BuyPass),
                BuyPass::existing_account_info_key(key.to_string(), staging).await?,
            ),
            Self::Custom(c, e) => (
                c.get_provider_type(),
                c.load_account_key_string(&e, staging).await?,
            ),
        };
        Ok(Acme::new(account, credentials, provider, handler, staging))
    }
}

impl Deref for Acme {
    type Target = AcmeProviderType;

    fn deref(&self) -> &Self::Target {
        &self.provider_type
    }
}

impl Acme {
    pub fn new(
        account: Account,
        credentials: AccountCredentials,
        provider: Arc<AcmeProviderType>,
        handler: Arc<impl AuthorizationHandler>,
        staging: bool,
    ) -> Self {
        Self {
            account: Arc::new(account),
            credentials: Arc::new(credentials),
            provider_type: provider,
            expired_days: RENEW_EXPIRED_DAYS,
            handler: handler.clone(),
            staging,
        }
    }
    pub fn set_expired_days(&mut self, days: usize) {
        if days < MIN_RENEW_EXPIRED_DAYS {
            self.expired_days = MIN_RENEW_EXPIRED_DAYS;
            return;
        }
        if days > MAX_RENEW_EXPIRED_DAYS {
            self.expired_days = MAX_RENEW_EXPIRED_DAYS;
            return;
        }
        self.expired_days = days;
    }
    pub fn get_handler(&self) -> Arc<dyn AuthorizationHandler> {
        self.handler.clone()
    }
    pub fn set_handler(&mut self, handler: Arc<impl AuthorizationHandler>) {
        self.handler = handler
    }
    pub fn get_provider_type(&self) -> Arc<AcmeProviderType> {
        self.provider_type.clone()
    }
    pub fn is_staging(&self) -> bool {
        self.staging
    }
    pub fn account(&self) -> Arc<Account> {
        self.account.clone()
    }
    pub fn credentials(&self) -> Arc<AccountCredentials> {
        self.credentials.clone()
    }
    pub async fn from_existing_account_provider(
        provider: impl AcmeProvider,
        handler: Arc<impl AuthorizationHandler>,
        staging: bool,
    ) -> ResultError<Self> {
        let (a, c) = provider.account_creation(true, staging).await?; // use existing account
        Ok(Self::new(
            a,
            c,
            provider.get_provider_type(),
            handler,
            staging,
        ))
    }
    pub async fn from_new_account_provider(
        provider: impl AcmeProvider,
        handler: Arc<impl AuthorizationHandler>,
        staging: bool,
    ) -> ResultError<Self> {
        let (a, c) = provider.account_creation(false, staging).await?; // create new account
        Ok(Self::new(
            a,
            c,
            provider.get_provider_type(),
            handler,
            staging,
        ))
    }

    async fn internal_request<V: Into<Vec<String>>>(
        &self,
        domains: V,
        identifiers: &[Identifier],
        cert_key: KeyPair,
    ) -> ResultError<CertificateResult> {
        let domains = domains.into();
        let new_order = NewOrder::new(&identifiers);
        let mut order = self
            .account
            .new_order(&new_order)
            .await
            .map_err(Error::from_acme_error)?;
        let mut params =
            CertificateParams::new(domains.clone()).map_err(|e| Error::other(e.to_string()))?;
        params.distinguished_name = rcgen::DistinguishedName::new();
        let csr = params
            .serialize_request(&cert_key)
            .map_err(|e| Error::other(e.to_string()))?;
        let mut handle = order.authorizations();
        let retry_policy = RetryPolicy::new();
        while let Some(auth) = handle.next().await {
            let mut auth: AuthorizationHandle = match auth {
                Ok(e) => e,
                Err(e) => return Err(Error::from_acme_error(e)),
            };
            let mut challenge_handle = match self.handler.handle(&mut auth).await {
                Ok(handler) => handler,
                Err(e) => {
                    if matches!(
                        auth.status,
                        AuthorizationStatus::Pending | AuthorizationStatus::Valid
                    ) {
                        auth.deactivate().await.ok();
                    }
                    return Err(e);
                }
            };
            if challenge_handle.status == ChallengeStatus::Pending {
                if let Err(e) = challenge_handle
                    .set_ready()
                    .await
                    .map_err(Error::from_acme_error)
                {
                    if matches!(
                        auth.status,
                        AuthorizationStatus::Pending | AuthorizationStatus::Valid
                    ) {
                        auth.deactivate().await.ok();
                    }
                    return Err(e);
                }
            }
            if let Some(err) = challenge_handle.error.clone() {
                if matches!(
                    auth.status,
                    AuthorizationStatus::Pending | AuthorizationStatus::Valid
                ) {
                    auth.deactivate().await.ok();
                }
                return Err(Error::from_acme_error(instant_acme::Error::Api(err)));
            }
            let status = challenge_handle.status;
            if status == ChallengeStatus::Valid {
                continue;
            }
            if matches!(
                auth.status,
                AuthorizationStatus::Pending | AuthorizationStatus::Valid
            ) {
                auth.deactivate().await.ok();
            }
            return Err(Error::other(format!(
                "Challenge cancelled by user with status: {:?}",
                status
            )));
        }

        let order_status = order
            .poll_ready(&retry_policy)
            .await
            .map_err(|e| Error::from_error(e))?;
        if order_status != OrderStatus::Ready {
            return Err(Error::other("Order is not ready"));
        }
        order
            .finalize_csr(csr.der())
            .await
            .map_err(Error::from_error)?;

        let fullchain_pem = order.certificate().await.map_err(Error::from_error)?;
        if let Some(chain) = fullchain_pem {
            return Ok(CertificateResult {
                csr,
                key_pair: cert_key.serialize_pem(),
                full_chain: chain.clone(),
                __certificates: OnceLock::new(),
            });
        }
        Err(Error::acme_failed(format!(
            "Can not get certificate for {:?}",
            domains
        )))
    }

    pub async fn request(
        &self,
        identity: RequestType,
        additional: Vec<RequestType>,
        key: Option<KeyPair>,
    ) -> ResultError<CertificateResult> {
        let mut set = HashMap::new();
        if !identity.is_valid() {
            return Err(Error::invalid_input(format!(
                "Requested {} is not valid",
                identity
            )));
        }
        set.insert(identity.to_string(), identity.clone());
        for i in additional.iter() {
            set.insert(i.to_string(), i.clone());
        }

        let mut new_domains = vec![];
        let mut identifiers = Vec::new();
        for (_, i) in set {
            let (i, identifier) = i.to_identifier_data()?;
            new_domains.push(i.clone());
            identifiers.push(identifier);
        }
        let identifiers: &[Identifier] = &identifiers;
        let mut cert_key;
        if let Some(k) = key {
            cert_key = k;
        } else {
            cert_key = KeyPair::generate().map_err(|e| Error::other(e.to_string()))?;
        }
        self.internal_request(new_domains, identifiers, cert_key)
            .await
    }

    pub async fn renew(
        &self,
        key: &str,
        cert: X509Certificate,
        force: bool,
    ) -> ResultError<Option<CertificateResult>> {
        let now = Utc::now();
        let threshold = now + Duration::days(self.expired_days as i64);
        let not_after = cert.validity_not_after();
        if !force {
            if not_after > threshold {
                return Ok(None); // don't renew
            }
        }
        let key = key.to_keypair()?;
        let pub_pem = cert.public_key_data();
        let pubkey_key_pem = key.public_key_raw().to_vec();
        if pub_pem != pubkey_key_pem {
            warn!(target: "acme", "X509 Certificate Public Key Mismatched");
        }
        let common_name = cert
            .subject_common_name()
            .ok_or_else(|| Error::other("Could not found old certificate subject name"))?;
        let mut identity = if Ip::public_ip_version(&common_name).is_some() {
            RequestType::Ip(common_name.clone())
        } else {
            RequestType::Domain(common_name.clone())
        };
        // OID Subject Alternative Name: 2.5.29.17
        let san_oid = &[2, 5, 29, 17];
        let mut additional = Vec::new();
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
                                    if san_str != common_name {
                                        additional.push(RequestType::Domain(san_str));
                                    }
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
                                if !ip_str.is_empty() && ip_str != common_name {
                                    additional.push(RequestType::Ip(ip_str));
                                }
                            }
                            _ => {} // skip
                        }
                    }
                    i = end;
                }
            }
        }
        Ok(Some(self.request(identity, additional, Some(key)).await?))
    }
}

#[derive(Debug, Clone)]
pub struct X509CertificateResult {
    pub certificate: X509Certificate,
    pub is_ca: bool,
}

#[derive(Debug, Clone)]
pub struct CertificateResult {
    pub csr: CertificateSigningRequest,
    pub key_pair: String,
    pub full_chain: String,
    __certificates: OnceLock<ResultError<Vec<X509CertificateResult>>>,
}

impl CertificateResult {
    pub fn get_leaf_certificates(&self) -> ResultError<Option<X509Certificate>> {
        self.get_certificates()
            .map(|certs| certs.into_iter().find(|i| !i.is_ca).map(|i| i.certificate))
    }

    pub fn get_certificates(&self) -> ResultError<Vec<X509CertificateResult>> {
        self.__certificates
            .get_or_init(|| {
                let mut reader = Cursor::new(&self.full_chain);
                let basic_constraints_oid = &[2, 5, 29, 19]; // OID: 2.5.29.19
                let mut certificates_vec = Vec::new();

                for x in certs(&mut reader) {
                    // 1. Handle PEM to DER error
                    let der = match x.map_err(Error::from_error) {
                        Ok(d) => d,
                        Err(e) => return Err(e),
                    };

                    // Parse DER to X509
                    let cert = match X509Certificate::from_der(der).map_err(Error::from_error) {
                        Ok(c) => c,
                        Err(e) => return Err(e),
                    };

                    // Check for CA flag in extensions
                    let mut is_ca = false;
                    for i in cert.iter_extensions() {
                        if i.id.as_ref() == basic_constraints_oid {
                            is_ca = true;
                            break;
                        }
                    }

                    certificates_vec.push(X509CertificateResult {
                        certificate: cert,
                        is_ca,
                    });
                }

                Ok(certificates_vec)
            })
            .clone()
    }
}
