use crate::cores::generator::random::Random;
use crate::cores::system::runtime::Runtime;
use serde::{Deserialize, Serialize};
use std::cmp::PartialEq;
use std::fmt::{Debug, Display, Formatter};
use std::iter::Iterator;
use std::ops::Deref;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Arc, LazyLock};
use totp_rs::{Algorithm, Rfc6238, Rfc6238Error, TotpUrlError, TOTP as TotpRs};

pub const STEAM_CHARS: &str = "23456789BCDFGHJKMNPQRTVWXY";
pub const DIGITS_CHARS: &str = "0123456780";
pub const STEAM_CHARS_VEC: LazyLock<Vec<char>> =
    LazyLock::new(|| STEAM_CHARS.chars().collect::<Vec<char>>());
pub const DIGIT_CHARS_VEC: LazyLock<Vec<char>> =
    LazyLock::new(|| DIGITS_CHARS.chars().collect::<Vec<char>>());

#[derive(Debug, Clone, Copy, Serialize, Deserialize, Default)]
pub enum TotpSkew {
    #[default]
    Default,
    Max,
    Min,
    Long,
    Steam,
    Google,
    Custom(u8),
}

impl TotpSkew {
    pub fn from_skew(skew: u8) -> Self {
        match skew {
            0 => Self::Min,
            1 => Self::Default,
            10 => Self::Max,
            5 => Self::Long,
            e => {
                let e = e.clamp(0, 10);
                Self::Custom(e)
            }
        }
    }
    pub fn to_skew(&self) -> u8 {
        match self {
            TotpSkew::Default | TotpSkew::Steam | TotpSkew::Google => 1,
            TotpSkew::Max => 10,
            TotpSkew::Min => 0,
            TotpSkew::Long => 5,
            TotpSkew::Custom(e) => (*e).clamp(0, 10),
        }
    }
}

impl PartialEq for TotpSkew {
    fn eq(&self, other: &Self) -> bool {
        self.to_skew() == other.to_skew()
    }
}
impl From<TotpSkew> for u8 {
    fn from(value: TotpSkew) -> Self {
        value.to_skew()
    }
}

impl From<TotpSkew> for usize {
    fn from(value: TotpSkew) -> Self {
        value.to_skew() as usize
    }
}

impl From<usize> for TotpSkew {
    fn from(value: usize) -> Self {
        let max = u8::MAX as usize;
        let value = value.clamp(0, max);
        TotpSkew::from_skew(value as u8)
    }
}

impl From<u8> for TotpSkew {
    fn from(value: u8) -> Self {
        TotpSkew::from_skew(value)
    }
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, Default)]
pub enum TotpDigit {
    #[default]
    Default,
    Max,
    Long,
    Min,
    Steam,
    Custom(usize),
}

impl TotpDigit {
    pub fn from_usize(size: usize) -> Self {
        match size {
            6 => Self::Default,
            8 => Self::Long,
            5 => Self::Min,
            10 => Self::Max,
            _ => Self::Custom(size.clamp(5, 10)),
        }
    }
    pub fn to_digit(&self) -> usize {
        match self {
            TotpDigit::Default => 6,
            TotpDigit::Long => 8,
            TotpDigit::Max => 10,
            TotpDigit::Steam | TotpDigit::Min => 5,
            TotpDigit::Custom(s) => (*s).clamp(5, 10),
        }
    }
}

impl PartialEq for TotpDigit {
    fn eq(&self, other: &Self) -> bool {
        self.to_digit() == other.to_digit()
    }
}
impl From<TotpDigit> for usize {
    fn from(value: TotpDigit) -> Self {
        value.to_digit()
    }
}

impl From<usize> for TotpDigit {
    fn from(value: usize) -> Self {
        TotpDigit::from_usize(value)
    }
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, Default)]
#[repr(u64)]
pub enum TotpStep {
    #[default]
    Default,
    Long,
    Max,
    Min,
    Custom(u64),
}

impl TotpStep {
    pub fn to_step(&self) -> u64 {
        match self {
            TotpStep::Default => 30,
            TotpStep::Long => 60,
            TotpStep::Max => 90,
            TotpStep::Min => 15,
            TotpStep::Custom(e) => (*e).clamp(15, 90),
        }
    }
    pub fn from_step(step: u64) -> Self {
        match step {
            15 => Self::Min,
            30 => Self::Default,
            60 => Self::Long,
            90 => Self::Max,
            _ => Self::Custom(step.clamp(15, 90)),
        }
    }
    pub fn filter_step(&self, step: u64) -> u64 {
        match step {
            15 | 30 | 60 => step,
            _ => step.clamp(15, 90),
        }
    }
}
impl PartialEq for TotpStep {
    fn eq(&self, other: &Self) -> bool {
        self.to_step() == other.to_step()
    }
}
impl From<TotpStep> for u64 {
    fn from(value: TotpStep) -> Self {
        value.to_step()
    }
}

impl From<u64> for TotpStep {
    fn from(value: u64) -> Self {
        TotpStep::from_step(value)
    }
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, Default)]
#[repr(u32)]
pub enum TotpCharLength {
    #[default]
    Default = 32, // 32 default
    Minimal = 26,
    Short = 16, // commonly steam
    Long = 64,
    Max = 103,
}

impl TotpCharLength {
    pub fn to_character_length(&self) -> u32 {
        *self as u32
    }
    pub fn to_byte_length(&self) -> u32 {
        match self {
            TotpCharLength::Default => 20,
            TotpCharLength::Minimal => 16,
            TotpCharLength::Short => 10,
            TotpCharLength::Long => 40,
            TotpCharLength::Max => 64,
        }
    }
    pub fn generate(&self) -> String {
        let length = self.to_character_length();
        Random::random_rfc4648(length)
    }
}

impl PartialEq for TotpCharLength {
    fn eq(&self, other: &Self) -> bool {
        self.to_character_length() == other.to_character_length()
    }
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, Default)]
pub enum TotpAlgorithm {
    #[default]
    SHA1,
    SHA256,
    SHA512,
    Steam,
    Google,
}

impl TotpAlgorithm {
    pub fn to_algorithm(&self) -> Algorithm {
        match self {
            Self::SHA1 => Algorithm::SHA1,
            Self::SHA256 => Algorithm::SHA256,
            Self::SHA512 => Algorithm::SHA512,
            Self::Steam => Algorithm::Steam,
            Self::Google => Algorithm::SHA1,
        }
    }
    pub fn identity(&self) -> String {
        let s = match self {
            Self::SHA1 => "SHA1",
            Self::SHA256 => "SHA256",
            Self::SHA512 => "SHA512",
            Self::Steam => "Steam",
            Self::Google => "Google",
        };
        s.to_string()
    }
    pub fn supported_characters(&self) -> &str {
        match self {
            Self::Steam => STEAM_CHARS,
            _ => DIGITS_CHARS,
        }
    }
    fn supported_characters_vec(&self) -> LazyLock<Vec<char>> {
        match self {
            Self::Steam => STEAM_CHARS_VEC,
            _ => DIGIT_CHARS_VEC,
        }
    }
    pub fn valid_characters(&self, value: &str) -> bool {
        let chars = self.supported_characters_vec();
        for x in value.chars() {
            if !chars.contains(&x) {
                return false;
            }
        }
        true
    }
    pub fn filter_digit(&self, digits: usize) -> TotpDigit {
        match self {
            Self::Steam => TotpDigit::Steam,
            Self::Google => TotpDigit::Default,
            _ => digits.into(),
        }
    }
    pub fn filter_skew(&self, skew: u8) -> TotpSkew {
        match self {
            Self::Steam | Self::Google => TotpSkew::Default,
            _ => skew.into(),
        }
    }
    pub fn filter_step(&self, step: u64) -> TotpStep {
        match self {
            Self::Steam | Self::Google => TotpStep::Default,
            _ => step.into(),
        }
    }
    pub fn digit(&self) -> TotpDigit {
        match self {
            Self::Steam => TotpDigit::Steam,
            _ => TotpDigit::Default,
        }
    }
    pub fn skew(&self) -> TotpSkew {
        TotpSkew::Default
    }
    pub fn step(&self) -> TotpStep {
        TotpStep::Default
    }
    pub fn to_key_char_length(&self) -> TotpCharLength {
        match self {
            Self::SHA512 => TotpCharLength::Max,
            Self::SHA256 => TotpCharLength::Long,
            Self::Steam => TotpCharLength::Short,
            _ => TotpCharLength::Default,
        }
    }
}

impl PartialEq for TotpAlgorithm {
    fn eq(&self, other: &Self) -> bool {
        self.identity() == other.identity()
    }
}

impl From<Algorithm> for TotpAlgorithm {
    fn from(value: Algorithm) -> Self {
        match value {
            Algorithm::SHA1 => Self::SHA1,
            Algorithm::SHA256 => Self::SHA256,
            Algorithm::SHA512 => Self::SHA512,
            Algorithm::Steam => Self::Steam,
        }
    }
}

impl From<&Algorithm> for TotpAlgorithm {
    fn from(value: &Algorithm) -> Self {
        Self::from(value.clone())
    }
}

impl From<&TotpAlgorithm> for TotpAlgorithm {
    fn from(value: &TotpAlgorithm) -> Self {
        value.clone()
    }
}

impl From<TotpAlgorithm> for Algorithm {
    fn from(value: TotpAlgorithm) -> Self {
        value.to_algorithm()
    }
}

impl From<&TotpAlgorithm> for Algorithm {
    fn from(value: &TotpAlgorithm) -> Self {
        value.to_algorithm()
    }
}

impl Deref for TotpAlgorithm {
    type Target = Algorithm;

    fn deref(&self) -> &Self::Target {
        match self {
            TotpAlgorithm::SHA1 => &Algorithm::SHA1,
            TotpAlgorithm::SHA256 => &Algorithm::SHA256,
            TotpAlgorithm::SHA512 => &Algorithm::SHA512,
            TotpAlgorithm::Steam => &Algorithm::Steam,
            TotpAlgorithm::Google => &Algorithm::SHA1,
        }
    }
}

impl Display for TotpAlgorithm {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.identity())
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TimeBasedOneTimePassword {
    pub algorithm: Arc<TotpAlgorithm>,
    pub step: TotpStep,
    pub digit: TotpDigit,
    pub char_length: TotpCharLength,
    pub skew: TotpSkew,
    pub default_issuer: Option<String>,
    pub default_account_name: String,
    __change: Arc<AtomicBool>,
}

impl TimeBasedOneTimePassword {
    pub fn new<I: Into<TotpAlgorithm>>(algorithm: I) -> Self {
        let algorithm = algorithm.into();
        let algorithm = Arc::new(algorithm);
        Self {
            step: algorithm.step(),
            skew: algorithm.skew(),
            digit: algorithm.digit(),
            char_length: algorithm.to_key_char_length(),
            algorithm,
            default_issuer: None,
            default_account_name: "totp".to_string(),
            __change: Arc::new(AtomicBool::new(false)),
        }
    }

    pub fn set_algorithm<I: Into<TotpAlgorithm>>(&mut self, algo: I) -> &mut Self {
        let algo = algo.into();
        let change = self.__change.load(Ordering::SeqCst);
        if !change {
            self.char_length = algo.to_key_char_length();
            self.skew = algo.skew();
            self.digit = algo.digit();
            self.step = algo.step();
        } else {
            self.__change.store(false, Ordering::SeqCst);
        }
        self.algorithm = Arc::new(algo);

        self
    }

    pub fn set_totp_digit<T: Into<TotpDigit>>(&mut self, size: T) -> &mut Self {
        let size = size.into();
        if size != self.digit {
            self.__change.store(true, Ordering::SeqCst);
        }
        self.digit = size;
        self
    }
    pub fn set_step<T: Into<TotpStep>>(&mut self, step: T) -> &mut Self {
        let step = step.into();
        if step != self.step {
            self.__change.store(true, Ordering::SeqCst);
        }
        self.step = step;
        self
    }
    pub fn set_skew<T: Into<TotpSkew>>(&mut self, skew: T) -> &mut Self {
        let skew = skew.into();
        if skew != self.skew {
            self.__change.store(true, Ordering::SeqCst);
        }
        self.skew = skew;
        self
    }
    pub fn set_totp_char_length<T: Into<TotpCharLength>>(&mut self, char_length: T) -> &mut Self {
        let char_length = char_length.into();
        if char_length != self.char_length {
            self.__change.store(true, Ordering::SeqCst);
        }
        self.char_length = char_length;
        self
    }

    pub fn set_default_issuer(&mut self, issuer: Option<String>) -> &mut Self {
        self.default_issuer = issuer;
        self
    }
    pub fn set_default_account_name<AccountName: AsRef<str>>(
        &mut self,
        account_name: AccountName,
    ) -> &mut Self {
        self.default_account_name = account_name.as_ref().to_string();
        self
    }
    pub fn generate_key(&self) -> String {
        self.char_length.generate()
    }
    fn convert_arg<Secret: AsRef<Vec<u8>>, AccountName: AsRef<str>>(
        &self,
        secret: Secret,
        issuer: Option<String>,
        account_name: AccountName,
        safe: bool,
    ) -> (Algorithm, usize, u8, u64, Vec<u8>, Option<String>, String) {
        let secret = secret.as_ref();
        let issuer = if let Some(issuer) = issuer {
            Some(issuer.clone())
        } else {
            None
        };
        let account_name = account_name.as_ref().to_string();
        let mut secrets: Vec<u8> = Vec::new();
        for v in secret {
            let i = *v;
            secrets.push(i);
        }
        let mut algorithm = self.algorithm.to_algorithm();
        let mut digit = self.digit.to_digit();
        let mut skew = self.skew.to_skew();
        let mut step = self.step.to_step();
        if safe {
            step = self.algorithm.filter_step(step).to_step();
            digit = self.algorithm.filter_digit(digit).to_digit();
            skew = self.algorithm.filter_skew(skew).to_skew();
        }
        (algorithm, digit, skew, step, secrets, issuer, account_name)
    }

    pub fn to_totp_with<Secret: AsRef<Vec<u8>>, AccountName: AsRef<str>>(
        &self,
        secret: Secret,
        issuer: Option<String>,
        account_name: AccountName,
    ) -> Result<TotpRs, TotpUrlError> {
        let (algorithm, digit, skew, step, secrets, issuer, account_name) =
            self.convert_arg(secret, issuer, account_name, false);
        TotpRs::new(algorithm, digit, skew, step, secrets, issuer, account_name)
    }

    pub fn to_safe_totp_with<Secret: AsRef<Vec<u8>>, AccountName: AsRef<str>>(
        &self,
        secret: Secret,
        issuer: Option<String>,
        account_name: AccountName,
    ) -> TotpRs {
        let (algorithm, digit, skew, step, secrets, issuer, account_name) =
            self.convert_arg(secret, issuer, account_name, true);
        TotpRs::new_unchecked(algorithm, digit, skew, step, secrets, issuer, account_name)
    }

    pub fn to_rfc6238_with<Secret: AsRef<Vec<u8>>, AccountName: AsRef<str>>(
        &self,
        secret: Secret,
        issuer: Option<String>,
        account_name: AccountName,
    ) -> Result<Rfc6238, Rfc6238Error> {
        let (_algorithm, digit, _skew, _step, secrets, issuer, account_name) =
            self.convert_arg(secret, issuer, account_name, false);
        Rfc6238::new(digit, secrets, issuer, account_name)
    }

    pub fn to_totp<Secret: AsRef<Vec<u8>>>(&self, secret: Secret) -> Result<TotpRs, TotpUrlError> {
        self.to_totp_with(
            secret,
            self.default_issuer.clone(),
            &self.default_account_name,
        )
    }

    pub fn to_safe_totp<Secret: AsRef<Vec<u8>>>(&self, secret: Secret) -> TotpRs {
        self.to_safe_totp_with(
            secret,
            self.default_issuer.clone(),
            &self.default_account_name,
        )
    }

    pub fn to_safe_totp_step<Secret: AsRef<Vec<u8>>, Step: Into<TotpStep>>(
        &self,
        secret: Secret,
        step: Step,
    ) -> TotpRs {
        let (algorithm, digit, skew, step, secrets, issuer, account_name) = self.convert_arg(
            secret,
            self.default_issuer.clone(),
            &self.default_account_name,
            true,
        );
        TotpRs::new_unchecked(algorithm, digit, skew, step, secrets, issuer, account_name)
    }

    pub fn to_rfc6238<Secret: AsRef<Vec<u8>>>(
        &self,
        secret: Secret,
    ) -> Result<Rfc6238, Rfc6238Error> {
        self.to_rfc6238_with(
            secret,
            self.default_issuer.clone(),
            &self.default_account_name,
        )
    }

    pub fn check_token<Token: AsRef<str>, Secret: AsRef<str>>(
        &self,
        token: Token,
        secret: Secret,
    ) -> bool {
        let secret = secret.as_ref().as_bytes().to_vec();
        let token = token.as_ref();
        if let Ok(e) = self.to_safe_totp(secret).check_current(token) {
            return e;
        }
        false
    }

    pub fn validate_with<Token: AsRef<str>, Secret: AsRef<str>, Step: Into<TotpStep>>(
        token: Token,
        secret: Secret,
        step: Step,
    ) -> bool {
        let secret = secret.as_ref().as_bytes().to_vec();
        let token = token.as_ref();
        if let Ok(e) = Self::default()
            .to_safe_totp_step(secret, step)
            .check_current(token)
        {
            return e;
        }
        false
    }
    pub fn validate<Token: AsRef<str>, Secret: AsRef<str>>(token: Token, secret: Secret) -> bool {
        let secret = secret.as_ref().as_bytes().to_vec();
        let token = token.as_ref();
        if let Ok(e) = Self::default().to_safe_totp(secret).check_current(token) {
            return e;
        }
        false
    }
}

impl Default for TimeBasedOneTimePassword {
    fn default() -> Self {
        Self {
            algorithm: Arc::new(TotpAlgorithm::default()),
            step: TotpStep::Default,
            digit: TotpDigit::Default,
            char_length: TotpCharLength::Default,
            skew: TotpSkew::Default,
            default_issuer: Some(Runtime::app_name().to_string()),
            default_account_name: Runtime::app_name().to_string().to_string(),
            __change: Arc::new(AtomicBool::new(false)),
        }
    }
}
