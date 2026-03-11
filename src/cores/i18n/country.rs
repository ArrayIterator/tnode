use log::warn;
use serde::Deserialize;
use std::collections::HashMap;
use std::sync::OnceLock;

/// {
//     "BD": {
//         "name": "Bangladesh",
//         "continent": {
//             "code": "AS",
//             "name": "Asia"
//         },
//         "code": {
//             "alpha2": "BD",
//             "alpha3": "BGD"
//         },
//         "numeric": "050",
//         "currencies": [
//             "BDT"
//         ],
//         "timezones": [
//             "Asia/Dhaka"
//         ]
//     }
// }
// ISO3166
///
#[derive(Deserialize, Debug, Clone)]
pub struct Continent {
    pub code: String,
    pub name: String,
}

#[derive(Deserialize, Debug, Clone)]
pub struct CountryCode {
    pub alpha2: String,
    pub alpha3: String,
}

#[derive(Deserialize, Debug, Clone)]
pub struct CountryData {
    pub name: String,
    pub continent: Continent,
    pub code: CountryCode,
    pub numeric: String,
    pub currencies: Vec<String>,
    pub timezones: Vec<String>,
}

static COUNTRY_DATA: OnceLock<HashMap<String, CountryData>> = OnceLock::new();

pub struct Country;

/// Country struct implements methods to get country data by code or name.
/// This data is based on the ISO 3166 standard.
impl Country {
    /// Get all country data
    /// # Example
    /// ```rust
    /// use crate::cores::i18n::country::Country;
    /// /// #[tokio::main]
    /// async fn main() {
    ///     let all_data = Country::all();
    ///     println!("Total countries: {}", all_data.len());
    /// }
    /// ```
    pub fn all() -> &'static HashMap<String, CountryData> {
        COUNTRY_DATA.get_or_init(|| {
            let json_str = include_str!("../../../resources/i18n/ISO3166.json");
            let raw_data: HashMap<String, CountryData> = serde_json::from_str(json_str)
                .unwrap_or_else(|e| {
                    warn!(target: "i18n", "Error parsing ISO3166.json: {}", e);
                    HashMap::new()
                });
            raw_data
                .into_iter()
                .map(|(k, v)| (k.to_uppercase(), v))
                // map code to uppercase
                .map(|(k, mut v)| {
                    // change code, continent code, and currencies to uppercase
                    v.code.alpha2 = v.code.alpha2.to_uppercase();
                    v.code.alpha3 = v.code.alpha3.to_uppercase();
                    v.continent.code = v.continent.code.to_uppercase();
                    v.currencies = v.currencies.into_iter().map(|c| c.to_uppercase()).collect();
                    (k, v)
                })
                .collect()
        })
    }

    /// Get country data by code (case-insensitive)
    /// # Example
    /// ```rust
    /// use crate::cores::i18n::country::Country;
    /// /// #[tokio::main]
    /// async fn main() {
    ///     let country_data = Country::get("us");
    ///     if let Some(data) = country_data {
    ///         println!("Country Name: {}", data.name);
    ///     }
    /// }
    /// ```
    pub fn get<T: AsRef<str>>(code: T) -> Option<&'static CountryData> {
        let code = code.as_ref().trim().to_uppercase();
        if code.len() != 2 {
            return None;
        }
        Self::all().get(&code)
    }

    /// Find country data by name (exact match, case-insensitive)
    /// # Example
    /// ```rust
    /// use crate::cores::i18n::country::Country;
    /// /// #[tokio::main]
    /// async fn main() {
    ///     let country_data = Country::find_by_country_name("Canada");
    ///     println!("Country Code: {}", country_data.code.alpha2);
    /// }
    /// ```
    pub fn find_by_country_name<T: AsRef<str>>(name: T) -> Option<&'static CountryData> {
        let name = name.as_ref().trim();
        if name.is_empty() {
            return None;
        }
        Self::all()
            .values()
            .find(|data| data.name.eq_ignore_ascii_case(&name))
    }

    /// Find country data by country code (exact match, case-insensitive)
    /// # Example
    /// ```rust
    /// use crate::cores::i18n::country::Country;
    /// /// #[tokio::main]
    /// async fn main() {
    ///     let country_data = Country::find_by_country_code("gb");
    ///     if let Some(data) = country_data {
    ///         println!("Country Name: {}", data.name);
    ///     }
    /// }
    /// ```
    pub fn find_by_country_code<T: AsRef<str>>(code: T) -> Option<&'static CountryData> {
        let code = code.as_ref().trim().to_uppercase();
        let length = code.len();
        if length == 2 {
            Self::all().values().find(|data| data.code.alpha2 == code)
        } else if length == 3 {
            Self::all().values().find(|data| data.code.alpha3 == code)
        } else {
            None
        }
    }
}
