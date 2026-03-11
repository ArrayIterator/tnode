use crate::cores::i18n::country::CountryData;
use log::warn;
use serde::Deserialize;
use std::collections::HashMap;
use std::sync::OnceLock;

/// {
//     "BDT": {
//         "code": "BDT",
//         "name": "Bangladeshi Taka",
//         "symbol": "\u09f3"
//     }
// }
// ISO4217
///
#[derive(Deserialize, Debug, Clone)]
pub struct CurrencyData {
    pub code: String,
    pub symbol: String,
    pub name: String,
}

pub struct Currency;

/// Currency data cache
static CURRENCY_DATA: OnceLock<HashMap<String, CurrencyData>> = OnceLock::new();

/// Currency struct implements methods to get currency data by code or name.
/// This data is based on the ISO 4217 standard.
impl Currency {
    /// Get all currency data
    /// # Example
    /// ```rust
    /// use crate::cores::i18n::currency::Currency;
    /// /// #[tokio::main]
    /// async fn main() {
    ///     let all_data = Currency::all();
    ///     println!("Total currencies: {}", all_data.len());
    /// }
    /// ```
    pub fn all() -> &'static HashMap<String, CurrencyData> {
        CURRENCY_DATA.get_or_init(||{
            let json_str = include_str!("../../../resources/i18n/ISO4217.json");
            let raw_data: HashMap<String, CurrencyData> = serde_json::from_str(json_str)
                .unwrap_or_else(|e| {
                    warn!(target: "i18n::Currency::all", "Error parsing ISO4217.json: {}", e);
                    HashMap::new()
                });
            raw_data
                .into_iter()
                .map(|(k, v)| (k.to_uppercase(), v))
                // map code to uppercase
                .map(|(k, mut v)| {
                    v.code = v.code.to_uppercase();
                    (k, v)
                })
                .collect()
        })
    }

    /// Get currency data by code (case-insensitive)
    /// # Example
    /// ```rust
    /// use crate::cores::i18n::currency::Currency;
    /// /// #[tokio::main]
    /// async fn main() {
    ///     if let Some(currency) = Currency::get("usd") {
    ///         println!("Currency Name: {}", currency.name);
    ///     }
    /// }
    /// ```
    pub fn get<T: AsRef<str>>(code: T) -> Option<&'static CurrencyData> {
        let code = code.as_ref().trim().to_uppercase();
        if code.len() != 3 {
            return None;
        }
        Self::all().get(&code)
    }

    /// Get currency data by country data
    /// # Example
    /// ```rust
    /// use crate::cores::i18n::currency::Currency;
    /// use crate::cores::i18n::country::Country;
    /// /// #[tokio::main]
    /// async fn main() {
    ///     let country = Country::get("US");
    ///     if let Some(currency) = Currency::from_country_data(country) {
    ///         println!("Currency Code: {}", currency.code);
    ///     }
    /// }
    /// ```
    pub fn from_country_data(country: CountryData) -> Option<&'static CurrencyData> {
        if let Some(code) = country.currencies.clone().pop() {
            Currency::get(code)
        } else {
            None
        }
    }

    /// Find currency data by symbol (exact match)
    /// # Example
    /// ```rust
    /// use crate::cores::i18n::currency::Currency;
    /// /// #[tokio::main]
    /// async fn main() {
    ///     if let Some(currency) = Currency::find_by_symbol("$") {
    ///         println!("Currency Code: {}", currency.code);
    ///     }
    /// }
    /// ```
    pub fn find_by_symbol<T: AsRef<str>>(symbol: T) -> Vec<&'static CurrencyData> {
        let symbol = symbol.as_ref().trim();
        if symbol.is_empty() {
            return vec![];
        }
        Self::all()
            .values()
            .filter(|c| c.symbol == symbol)
            .collect()
    }

    /// Find currency data by name (case-insensitive, partial match)
    /// # Example
    /// ```rust
    /// use crate::cores::i18n::currency::Currency;
    /// /// #[tokio::main]
    /// async fn main() {
    ///     let results = Currency::find_by_name("dollar");
    ///     for currency in results {
    ///         println!("Found Currency: {} ({})", currency.name, currency.code);
    ///     }
    /// }
    /// ```
    pub async fn find_by_name<T: AsRef<str>>(name: T) -> Vec<&'static CurrencyData> {
        // trim
        let name = name.as_ref().trim().to_lowercase();
        if name.is_empty() {
            return vec![];
        }
        Self::all()
            .values()
            .filter(|c| c.name.to_lowercase().contains(&name))
            .collect()
    }
}
