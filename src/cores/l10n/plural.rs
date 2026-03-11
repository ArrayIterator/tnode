///
/// Plural resolution utilities following CLDR-style rules.
///
/// This module provides:
/// - `PluralCategory`: enum describing supported plural rule families.
/// - `PluralResolver`: helper with `get_index` to map a locale and a numeric
///   value to a plural form index.
/// - `ToPluralCount`: trait to convert common numeric types into `f64` for
///   uniform plural-rule evaluation.
/// - `split_plural`: helper to split a null-separated plural forms string.
///

/// Plural categories according to CLDR standards (grouping only).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PluralCategory {
    /// Languages without plural forms (same form for all counts).
    Only,
    /// Languages with "one" vs "other" (standard western).
    OneOther,
    /// Slavic-style plurals (three forms, distinct rules).
    Slavic,
    /// Baltic-style plurals (Lithuanian-style rules).
    Baltic,
    /// Arabic-style plurals (six-way classification).
    Arabic,
}


/// 
/// A trait that provides a method to convert a numeric type into a non-negative
/// `f64` value for use in plural rule evaluation. This trait is intended to be
/// implemented by numeric types that can be safely and meaningfully converted
/// to an `f64` value for this purpose.
///
/// # Purpose
/// The `ToPluralCount` trait enables numeric types to participate in plural
/// rule evaluation by abstractly converting themselves into an appropriate
/// floating-point representation.
///
/// # Required Method
/// - `to_f64`: Converts the numeric value into a non-negative `f64`. This
///   conversion ensures that the resulting value is suitable for use in
///   contexts where plural rule evaluation is required.
///
/// # Visibility
/// This trait is scoped to the current crate (`pub(crate)`), meaning it is
/// intended for internal usage within the defining crate and is not exposed
/// publicly to external crates.
///
/// # Example Usage
/// ```rust
/// // Example implementation of the `ToPluralCount` trait for an i32.
/// impl ToPluralCount for i32 {
///     fn to_f64(self) -> f64 {
///         self.max(0) as f64 // Ensure non-negative value
///     }
/// }
///
/// let count: i32 = -5;
/// assert_eq!(count.to_f64(), 0.0);
///
/// pub trait ToPluralCount {
///     /// Converts the implementing numeric type into a non-negative `f64`
///     /// suitable for plural rule evaluation.
///     ///
///     /// # Returns
///     /// - A `f64` representation of the numeric value that is guaranteed to be
///     ///   non-negative.
///     ///
///     /// # Note
///     /// Implementations of this method should ensure that the resulting `f64`
///     /// does not represent a negative value, as this is a precondition for
///     /// plural rule evaluation.
///     fn to_f64(self) -> f64;
/// }
/// ```
pub trait ToPluralCount {
    /// Convert the implementing numeric type into a non-negative `f64`
    /// suitable for plural rule evaluation.
    fn to_f64(self) -> f64;
}

impl ToPluralCount for isize {
    fn to_f64(self) -> f64 {
        self.abs() as f64
    }
}

impl ToPluralCount for i32 {
    fn to_f64(self) -> f64 {
        self.abs() as f64
    }
}
impl ToPluralCount for i64 {
    fn to_f64(self) -> f64 {
        self.abs() as f64
    }
}
impl ToPluralCount for f32 {
    fn to_f64(self) -> f64 {
        self.abs() as f64
    }
}
impl ToPluralCount for f64 {
    fn to_f64(self) -> f64 {
        self.abs()
    }
}

/// A struct representing the concept of pluralization.
///
/// The `Plural` struct serves as a marker or utility for managing pluralization
/// concepts in the application. It has a `Debug` implementation derived for
/// convenient debugging and logging purposes.
///
/// # Visibility
/// - This struct is restricted to the current crate due to its `pub(crate)` visibility modifier.
///
/// # Example
/// ```
/// use crate::Plural;
///
/// let plural = Plural;
/// println!("{:?}", plural); // Outputs: Plural
/// ```
///
/// # Derives
/// - `Debug`: Automatically provides formatting for debugging output.
///
/// # See Also
/// - Additional utilities or functions may interact with the `Plural` struct
///   for handling pluralization scenarios in the application.
#[derive(Debug)]
pub struct Plural;

/// implementation of the `Plural` struct
impl Plural {
    /// Determine the plural form index for a given `locale` and numeric `n`.
    ///
    /// - `locale`: locale identifier (examples: `en-US`, `pl_PL`). Hyphens are
    ///   normalized to underscores and matching is case-insensitive.
    /// - `n`: numeric count; any type implementing `ToPluralCount` is accepted
    ///   (integers and floats).
    ///
    /// Returns a `usize` index into the plural forms vector (0-based). The
    /// mapping implements CLDR-style rules for the supported language groups
    /// (no plural, one/other, Slavic, Polish, Baltic, Arabic). Unknown locales
    /// use a default one/other fallback.
    pub fn resolve<L: AsRef<str>, N: ToPluralCount>(locale: L, n: N) -> usize {
        let val = n.to_f64();
        let loc = locale.as_ref();

        // Fast normalization without full string allocation if possible.
        // Replace '-' with '_' to support both `en-US` and `en_US`.
        let normalized = if loc.contains('-') {
            loc.replace('-', "_")
        } else {
            loc.to_string()
        };
        match normalized.to_lowercase().as_str() {
            // Group 1: No plural (Indonesian, etc.)
            "id_id" | "ms_my" | "vi_vn" | "zh_cn" | "ja_jp" | "ko_kr" | "th_th" | "tr_tr" => 0,

            // Group 2: one vs other (standard western)
            "en_us" | "en_gb" | "de_de" | "fr_fr" | "es_es" | "it_it" | "nl_nl" | "pt_br"
            | "pt_pt" | "sv_se" | "nb_no" | "da_dk" | "fi_fi" | "el_gr" | "hi_in" => {
                if val == 1.0 {
                    0
                } else {
                    1
                }
            }

            // Group 3: Slavic (Russian, etc.)
            "ru_ru" | "uk_ua" | "be_by" | "sr_rs" | "hr_hr" | "bs_ba" => {
                if val.fract() != 0.0 {
                    2 // Decimals usually take the 'other' (plural) form
                } else {
                    let i = val as i64;
                    if i % 10 == 1 && i % 100 != 11 {
                        0
                    } else if i % 10 >= 2 && i % 10 <= 4 && (i % 100 < 10 || i % 100 >= 20) {
                        1
                    } else {
                        2
                    }
                }
            }

            // Group 4: Polish
            "pl_pl" => {
                if val.fract() != 0.0 {
                    2
                } else {
                    let i = val as i64;
                    if i == 1 {
                        0
                    } else if i % 10 >= 2 && i % 10 <= 4 && (i % 100 < 10 || i % 100 >= 20) {
                        1
                    } else {
                        2
                    }
                }
            }

            // Group 5: Baltic (Lithuanian)
            "lt_lt" => {
                if val.fract() != 0.0 {
                    2
                } else {
                    let i = val as i64;
                    if i % 10 == 1 && i % 100 != 11 {
                        0
                    } else if i % 10 >= 2 && (i % 100 < 10 || i % 100 >= 20) {
                        1
                    } else {
                        2
                    }
                }
            }

            // Group 6: Arabic
            "ar_sa" | "ar_eg" | "ar_ae" | "ar_jo" | "ar_iq" | "ar_ps" => {
                if val.fract() != 0.0 {
                    5 // Other
                } else {
                    let i = val as i64;
                    if i == 0 {
                        0
                    } else if i == 1 {
                        1
                    } else if i == 2 {
                        2
                    } else if i % 100 >= 3 && i % 100 <= 10 {
                        3
                    } else if i % 100 >= 11 && i % 100 <= 99 {
                        4
                    } else {
                        5
                    }
                }
            }

            // Default fallback: one/other
            _ => {
                if val == 1.0 {
                    0
                } else {
                    1
                }
            }
        }
    }

    /// Split a null-separated plural forms string into owned `String` parts.
    /// This for translation splitter in 1 table of database
    ///
    /// Example: `"one\0other\0many"` -> `vec!["one".to_string(), "other".to_string(), "many".to_string()]`
    pub fn split_plural<T: AsRef<str>>(plural: T) -> Vec<String> {
        let p = plural.as_ref();
        if p.is_empty() {
            return vec![];
        }

        p.split('\0').map(|s| s.to_string()).collect()
    }
}
