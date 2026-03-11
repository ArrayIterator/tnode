use crate::cores::idna::domain::Domain;
use crate::cores::system::error::{Error, ResultError};
use instant_acme::{Account, AccountCredentials, ExternalAccountKey, Key, NewAccount};
use rcgen::KeyPair;
use rustls_pki_types::{PrivateKeyDer, PrivatePkcs8KeyDer};
use std::fmt::Debug;
use std::fs;
use std::ops::Deref;
use std::path::Path;
use std::sync::Arc;

#[derive(Debug, Clone)]
pub enum AcmeProviderType {
    LetsEncrypt,
    Google,
    ZeroSSL,
    BuyPass,
    // name, acme_url, staging_url, wildcard support, ip support, required eab
    Custom(String, String, Option<String>, bool, bool, bool),
}

impl AcmeProviderType {
    pub fn get_name(&self) -> &str {
        match self {
            AcmeProviderType::LetsEncrypt => "Let's Encrypt",
            AcmeProviderType::Google => "Google Trust Service",
            AcmeProviderType::ZeroSSL => "ZeroSSL",
            AcmeProviderType::BuyPass => "BuyPass",
            AcmeProviderType::Custom(name, ..) => &name,
        }
    }
    pub fn get_acme_url(&self) -> &str {
        match self {
            AcmeProviderType::LetsEncrypt => "https://acme-v02.api.letsencrypt.org/directory",
            AcmeProviderType::Google => "https://dv.acme-v02.api.pki.goog/directory",
            AcmeProviderType::ZeroSSL => "https://acme.zerossl.com/v2/DV90",
            AcmeProviderType::BuyPass => "https://api.buypass.com/acme/directory",
            AcmeProviderType::Custom(_, prod, ..) => &prod,
        }
    }
    pub fn get_acme_staging_url(&self) -> Option<&str> {
        match self {
            AcmeProviderType::LetsEncrypt => {
                Some("https://acme-staging-v02.api.letsencrypt.org/directory")
            }
            AcmeProviderType::Google => Some("https://dv.acme-v02.test-api.pki.goog/directory"),
            AcmeProviderType::ZeroSSL => None,
            AcmeProviderType::BuyPass => Some("https://api.test.buypass.com/acme/directory"),
            AcmeProviderType::Custom(_, _, s, ..) => {
                if let Some(s) = s {
                    Some(&s)
                } else {
                    None
                }
            }
        }
    }
    pub fn is_wildcard_support(&self) -> bool {
        match self {
            AcmeProviderType::LetsEncrypt => true,
            AcmeProviderType::Google => true,
            AcmeProviderType::ZeroSSL => true,
            AcmeProviderType::BuyPass => false,
            AcmeProviderType::Custom(_, _, _, b, ..) => *b,
        }
    }
    pub fn is_ip_support(&self) -> bool {
        match self {
            AcmeProviderType::LetsEncrypt => false,
            AcmeProviderType::Google => false,
            AcmeProviderType::ZeroSSL => true,
            AcmeProviderType::BuyPass => false,
            AcmeProviderType::Custom(_, _, _, _, b, _) => *b,
        }
    }
    pub fn is_required_eab(&self) -> bool {
        match self {
            AcmeProviderType::LetsEncrypt => false,
            AcmeProviderType::Google => false,
            AcmeProviderType::ZeroSSL => true,
            AcmeProviderType::BuyPass => false,
            AcmeProviderType::Custom(_, _, _, _, _, b) => *b,
        }
    }
}

#[derive(Debug, Clone)]
pub struct ExternalAccountBinding {
    pub id: String,
    pub hmac_key: String,
}

impl ExternalAccountBinding {
    pub fn new<T: AsRef<str>, H: AsRef<str>>(id: T, hmac: H) -> Self {
        Self {
            id: id.as_ref().to_string(),
            hmac_key: hmac.as_ref().to_string(),
        }
    }
    pub fn to_external_account_key(&self) -> ExternalAccountKey {
        ExternalAccountKey::new(self.id.clone(), self.hmac_key.as_bytes())
    }
}

#[async_trait::async_trait]
pub trait AcmeProvider: Debug + Send + Sync + 'static {
    fn get_email(&self) -> &str;
    fn get_eab(&self) -> Option<ExternalAccountBinding>;
    fn get_provider_type(&self) -> Arc<AcmeProviderType>;
    fn get_acme_url_of(&self, staging: bool) -> ResultError<String> {
        let d = if !staging {
            self.get_provider_type().get_acme_url().to_string()
        } else if let Some(acme) = &self.get_provider_type().get_acme_staging_url() {
            acme.to_string()
        } else {
            return Err(Error::unsupported(format!(
                "{} does not support acme staging",
                self.get_provider_type().get_name()
            )));
        };
        Ok(d)
    }

    async fn account_creation(
        &self,
        only_return_existing: bool,
        staging: bool,
    ) -> ResultError<(Account, AccountCredentials)> {
        let eab = if let Some(eab) = &self.get_eab() {
            Some(&eab.to_external_account_key())
        } else {
            None
        };
        let acme_url = self.get_acme_url_of(staging)?;
        Account::builder()
            .map_err(Error::from_acme_error)?
            .create(
                &NewAccount {
                    contact: &[&self.get_email()],
                    terms_of_service_agreed: true,
                    only_return_existing,
                },
                acme_url.clone(),
                eab,
            )
            .await
            .map_err(Error::from_acme_error)
    }

    fn create_keys_from_string(
        &self,
        key: &str,
    ) -> ResultError<(PrivatePkcs8KeyDer<'static>, Key, PrivateKeyDer<'static>)> {
        let key_bytes = key.as_bytes();
        let pkcs8 = PrivatePkcs8KeyDer::from(key_bytes);
        Ok((
            pkcs8.clone_key(),
            Key::from_pkcs8_der(pkcs8.clone_key()).map_err(Error::from_acme_error)?,
            PrivateKeyDer::Pkcs8(pkcs8.clone_key()),
        ))
    }

    //noinspection DuplicatedCode
    async fn load_account_pkcs8(
        &self,
        pkcs8: PrivatePkcs8KeyDer<'static>,
        staging: bool,
    ) -> ResultError<(Account, AccountCredentials)> {
        let acme = self.get_acme_url_of(staging)?;
        Account::builder()
            .map_err(Error::from_acme_error)?
            .from_key(
                (
                    Key::from_pkcs8_der(pkcs8.clone_key()).map_err(Error::from_acme_error)?,
                    PrivateKeyDer::Pkcs8(pkcs8.clone_key()),
                ),
                acme.to_string(),
            )
            .await
            .map_err(Error::from_acme_error)
    }

    async fn load_account_key_string(
        &self,
        key: &str,
        staging: bool,
    ) -> ResultError<(Account, AccountCredentials)> {
        let acme = self.get_acme_url_of(staging)?;
        let (_, key, der) = self.create_keys_from_string(&key)?;
        Account::builder()
            .map_err(Error::from_acme_error)?
            .from_key((key, der), acme.to_string())
            .await
            .map_err(Error::from_acme_error)
    }

    async fn load_account_from_key_file(
        &self,
        path: &Path,
        staging: bool,
    ) -> ResultError<(Account, AccountCredentials)> {
        let key = fs::read_to_string(&path).map_err(Error::from_io_error)?;
        Ok(self.load_account_key_string(&key, staging).await?)
    }
}

macro_rules! create_instance_provider {
    (
        $provider:ident,
        $provider_type:expr
    ) => {
        #[derive(Debug, Clone)]
        pub struct $provider {
            pub email: String,
            pub eab: Option<ExternalAccountBinding>,
            pub provider: Arc<AcmeProviderType>,
        }
        impl $provider {
            pub fn new(email: &str, eab: Option<ExternalAccountBinding>) -> ResultError<Self>
            where
                Self: Sized,
            {
                if $provider_type.is_required_eab() && eab.is_none() {
                    return Err(Error::invalid_input(format!(
                        "External Account Binding (EAB) is required for: {}",
                        $provider_type.get_name()
                    )));
                }
                let email_address = Domain::parse_email(email)?.to_lowercase();
                Ok(Self {
                    email: email_address,
                    eab: eab.clone(),
                    provider: Arc::new($provider_type),
                })
            }

            pub async fn existing_account_info_key(
                key: String,
                staging: bool,
            ) -> ResultError<(Account, AccountCredentials)>
            where
                Self: Sized,
            {
                let acme_url = if staging {
                    if let Some(uri) = $provider_type.get_acme_staging_url() {
                        uri
                    } else {
                        return Err(Error::acme_failed(format!(
                            "Acme {} does not support staging",
                            $provider_type.get_name()
                        )));
                    }
                } else {
                    $provider_type.get_acme_url()
                };
                let key_bytes = key.as_bytes();
                let pkcs8 = PrivatePkcs8KeyDer::from(key_bytes);
                Account::builder()
                    .map_err(Error::from_acme_error)?
                    .from_key(
                        (
                            Key::from_pkcs8_der(pkcs8.clone_key())
                                .map_err(Error::from_acme_error)?,
                            PrivateKeyDer::Pkcs8(pkcs8.clone_key()),
                        ),
                        acme_url.to_string(),
                    )
                    .await
                    .map_err(Error::from_acme_error)
            }
        }
        impl Deref for $provider {
            type Target = AcmeProviderType;
            fn deref(&self) -> &Self::Target {
                &self.provider
            }
        }
        #[async_trait::async_trait]
        impl AcmeProvider for $provider {
            fn get_provider_type(&self) -> Arc<AcmeProviderType> {
                self.provider.clone()
            }
            fn get_email(&self) -> &str {
                &self.email
            }
            fn get_eab(&self) -> Option<ExternalAccountBinding> {
                self.eab.clone()
            }
        }
    };
}

create_instance_provider!(LetsEncrypt, AcmeProviderType::LetsEncrypt);
create_instance_provider!(Google, AcmeProviderType::Google);
create_instance_provider!(ZeroSSL, AcmeProviderType::ZeroSSL);
create_instance_provider!(BuyPass, AcmeProviderType::BuyPass);

#[derive(Debug)]
pub struct CustomProvider {
    pub email: String,
    pub eab: Option<ExternalAccountBinding>,
    pub provider: Arc<AcmeProviderType>,
}

impl CustomProvider {
    pub fn new(
        provider: Arc<AcmeProviderType>,
        email: &str,
        eab: Option<ExternalAccountBinding>,
    ) -> ResultError<Self> {
        if let None = eab
            && provider.is_required_eab()
        {
            return Err(Error::invalid_input(format!(
                "External Account Binding (EAB) is required for custom provider: {}",
                provider.get_name()
            )));
        }
        Ok(Self {
            email: email.to_string(),
            eab,
            provider: provider.clone(),
        })
    }

    //noinspection DuplicatedCode
    pub async fn existing_account_info_key(
        key: String,
        acme_url: &'static str,
    ) -> ResultError<(Account, AccountCredentials)>
    where
        Self: Sized,
    {
        let key_bytes = key.as_bytes();
        let pkcs8 = PrivatePkcs8KeyDer::from(key_bytes);
        Account::builder()
            .map_err(Error::from_acme_error)?
            .from_key(
                (
                    Key::from_pkcs8_der(pkcs8.clone_key()).map_err(Error::from_acme_error)?,
                    PrivateKeyDer::Pkcs8(pkcs8.clone_key()),
                ),
                acme_url.to_string(),
            )
            .await
            .map_err(Error::from_acme_error)
    }
}

impl AcmeProvider for CustomProvider {
    fn get_email(&self) -> &str {
        &self.email
    }
    fn get_eab(&self) -> Option<ExternalAccountBinding> {
        self.eab.clone()
    }
    fn get_provider_type(&self) -> Arc<AcmeProviderType> {
        self.provider.clone()
    }
}

impl Deref for CustomProvider {
    type Target = AcmeProviderType;

    fn deref(&self) -> &Self::Target {
        &self.provider
    }
}

#[derive(Debug)]
pub enum AcmeProviderCompartment {
    LetsEncrypt(String, Option<ExternalAccountBinding>),
    Google(String, ExternalAccountBinding),
    ZeroSSL(String, ExternalAccountBinding),
    BuyPass(String, Option<ExternalAccountBinding>),
}

#[derive(Debug)]
pub enum AcmeInstance {
    LetsEncrypt(LetsEncrypt),
    Google(Google),
    ZeroSSL(ZeroSSL),
    BuyPass(BuyPass),
}

impl Deref for AcmeInstance {
    type Target = dyn AcmeProvider;

    fn deref(&self) -> &Self::Target {
        match self {
            AcmeInstance::LetsEncrypt(e) => e,
            AcmeInstance::Google(e) => e,
            AcmeInstance::ZeroSSL(e) => e,
            AcmeInstance::BuyPass(e) => e,
        }
    }
}

impl AcmeProviderCompartment {
    pub fn to_provider(&self) -> ResultError<AcmeInstance> {
        Ok(match self {
            AcmeProviderCompartment::LetsEncrypt(email, eab) => {
                AcmeInstance::LetsEncrypt(LetsEncrypt::new(&email, eab.clone())?)
            }
            AcmeProviderCompartment::ZeroSSL(email, eab) => {
                AcmeInstance::ZeroSSL(ZeroSSL::new(&email, Some(eab.clone()))?)
            }
            AcmeProviderCompartment::BuyPass(email, eab) => {
                AcmeInstance::BuyPass(BuyPass::new(&email, eab.clone())?)
            }
            AcmeProviderCompartment::Google(email, eab) => {
                AcmeInstance::Google(Google::new(&email, Some(eab.clone()))?)
            }
        })
    }
}

pub trait KeyPairConversion {
    fn to_keypair(&self) -> ResultError<KeyPair>;
}

impl<T: AsRef<[u8]>> KeyPairConversion for T {
    fn to_keypair(&self) -> ResultError<KeyPair> {
        let str = String::from_utf8(self.as_ref().to_vec()).map_err(Error::parse_error)?;
        Ok(KeyPair::from_pem(&str).map_err(Error::from_error)?)
    }
}
