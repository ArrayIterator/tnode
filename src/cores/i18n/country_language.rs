use log::{debug, warn};
use serde::Deserialize;
use std::collections::HashMap;
use std::sync::OnceLock;

use crate::cores::i18n::country::CountryData;

/// "BD": {
//         "code": "BN",
//         "locale": "bn_BD",
//         "english": "Bengali",
//         "original": "বাংলা",
//         "dir": "ltr"
//     }
// }
// ISO3166 with language info
///
#[derive(Deserialize, Debug, Clone)]
pub struct CountryLanguageData {
    // language code, e.g., "BN"
    pub code: String,
    // locale, e.g., "bn_BD"
    pub locale: String,
    // English name of the language, e.g., "Bengali"
    pub english: String,
    // Original name of the language, e.g., "বাংলা"
    pub original: String,
    // text direction, e.g., "ltr" or "rtl"
    pub dir: String,
}

impl CountryLanguageData {
    /// Converts the locale string stored in the `locale` field of the struct
    /// by replacing all underscores (`_`) with hyphens (`-`).
    ///
    /// # Returns
    ///
    /// A `String` containing the modified locale code where underscores
    /// are replaced by hyphens.
    ///
    /// # Example
    ///
    /// ```rust
    /// let my_struct = MyStruct { locale: String::from("en_US") };
    /// let locale = my_struct.locale_code();
    /// assert_eq!(locale, "en-US");
    /// ```
    ///
    /// This function is typically used to standardize locale codes for systems
    /// that prefer hyphen-separated formats (e.g., `en-US` instead of `en_US`).
    pub fn locale_code(&self) -> String {
        self.locale.replace("_", "-")
    }

    /// Returns the `code` property of the struct as a `String`.
    ///
    /// # Example
    ///
    /// ```rust
    /// let instance = YourStruct { code: String::from("example_code") };
    /// let code = instance.code();
    /// assert_eq!(code, "example_code");
    /// ```
    ///
    /// # Returns
    /// * `String` - A clone of the `code` field.
    pub fn code(&self) -> String {
        self.code.clone()
    }
}

static COUNTRY_LANGUAGE_DATA: OnceLock<HashMap<String, CountryLanguageData>> = OnceLock::new();

pub struct CountryLanguage;

/// Country Language that implements methods to get country language data by country code.
impl CountryLanguage {
    /// Normalize country code to 2 digits code.
    /// # Example
    /// ```rust
    /// use crate::cores::i18n::country_language::CountryLanguage;
    /// /// #[tokio::main]
    /// async fn main() {
    ///     let code = CountryLanguage::normalize_code_2("BD");
    ///     println!("Normalized code: {}", code);
    /// }
    /// ```
    pub fn normalize_code_2<T: AsRef<str>>(code: T) -> String {
        let mut code = code.as_ref().to_string();
        // return 2 number code
        if code.len() > 2 {
            code = code[..2].to_string();
        }
        code.to_uppercase()
    }

    /// Get all country language data - key as country code
    /// # Example
    /// ```rust
    /// use crate::cores::i18n::country_language::CountryLanguage;
    /// /// #[tokio::main]
    /// async fn main() {
    ///    let all_data = CountryLanguage::all();
    ///   println!("Total country languages: {}", all_data.len());
    /// }
    /// ```
    pub fn all() -> &'static HashMap<String, CountryLanguageData> {
        COUNTRY_LANGUAGE_DATA.get_or_init(|| {
            debug!(target: "i18n", "Loading ISO-3166-language.json data...");
            let json_str = include_str!("../../../resources/i18n/ISO-3166-language.json");
            let raw_data: HashMap<String, CountryLanguageData> = serde_json::from_str(json_str)
                .unwrap_or_else(|e| {
                    warn!(target: "i18n", "Error parsing ISO-3166-language.json: {}", e);
                    HashMap::new()
                });
            raw_data
                .into_iter()
                .map(|(k, v)| (k.to_uppercase(), v))
                // map code to uppercase
                .map(|(k, mut v)| {
                    v.code = v.code.to_uppercase();
                    // normalize locale to xx_XX format
                    let parts: Vec<&str> = v.locale.split('_').collect();
                    if parts.len() == 2 {
                        v.locale =
                            format!("{}_{}", parts[0].to_lowercase(), parts[1].to_uppercase());
                    }
                    (k, v)
                })
                .collect()
        })
    }

    /// Get country language data by country data
    /// # Example
    /// ```rust
    /// use crate::cores::i18n::country_language::CountryLanguage;
    /// use crate::cores::i18n::cuntry::Country;
    /// /// #[tokio::main]
    /// async fn main() {
    ///     let country = Country::get("US")?;
    ///     if let Some(data) = CountryLanguage::from_country_data(country) {
    ///         println!("Language: {}", data.english);
    ///     }
    /// }
    /// ```
    pub fn from_country_data(country: CountryData) -> Option<&'static CountryLanguageData> {
        Self::get(country.code.alpha2)
    }

    /// Get country language data by country code (case-insensitive)
    /// # Example
    /// ```rust
    /// use crate::cores::i18n::country_language::CountryLanguage;
    /// /// #[tokio::main]
    /// async fn main() {
    ///     if let Some(data) = CountryLanguage::get("bd") {
    ///         println!("Language: {}", data.english);
    ///     }
    /// }
    /// ```
    pub fn get<T: AsRef<str>>(code: T) -> Option<&'static CountryLanguageData> {
        let code = code.as_ref();
        if code.len() != 2 {
            return None;
        }
        Self::all().get(&code.to_uppercase())
    }

    /// Find country language data by language name (exact match, case-insensitive)
    /// # Example
    /// ```rust
    /// use crate::cores::i18n::country_language::CountryLanguage;
    /// /// #[tokio::main]
    /// async fn main() {
    ///     let results = CountryLanguage::find_by_language("Bengali");
    ///     for data in results {
    ///         println!("Country Code: {}", data.code);
    ///     }
    /// }
    /// ```
    pub fn find_by_language<T: AsRef<str>>(language: T) -> Vec<&'static CountryLanguageData> {
        let lang = language.as_ref();
        if lang.len() < 2 {
            return vec![];
        }
        let lang = lang.to_lowercase();
        Self::all()
            .values()
            .filter(|data| {
                data.english.eq_ignore_ascii_case(&lang)
                    || data.original.eq_ignore_ascii_case(&lang)
            })
            .collect()
    }

    /// Find country language data by locale (exact match, case-insensitive)
    /// # Example
    /// ```rust
    /// use crate::cores::i18n::country_language::CountryLanguage;
    /// /// #[tokio::main]
    /// async fn main() {
    ///     let results = CountryLanguage::find_by_locale("bn_BD");
    ///     for data in results {
    ///         println!("Language: {}", data.english);
    ///     }
    /// }
    /// ```
    pub fn find_by_locale<T: AsRef<str>>(locale: T) -> Vec<&'static CountryLanguageData> {
        let locale = locale.as_ref();
        if locale.len() < 2 {
            return vec![];
        }
        let locale = locale.to_lowercase();
        Self::all()
            .values()
            .filter(|data| data.locale.eq_ignore_ascii_case(&locale))
            .collect()
    }
}
