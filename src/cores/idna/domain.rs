use crate::cores::idna::psl::PSL;
use crate::cores::system::error::{Error, ResultError};
use regex::Regex;
use std::sync::LazyLock;
use url::quirks::{domain_to_ascii, domain_to_unicode};
use url::Url;

static TLD_REGEX: LazyLock<Regex> =
    LazyLock::new(|| Regex::new(r"^(?![0-9]+$)[a-z0-9-]+$").expect("Invalid Regex"));
static PROTOCOL_REGEX: LazyLock<Regex> =
    LazyLock::new(|| Regex::new(r"^(?i)([a-z0-9+.-]+)://").expect("Invalid Protocol Regex"));

#[derive(Debug)]
pub struct Domain;

#[derive(Debug, Clone)]
pub struct DomainSpec {
    // full domain name, including subdomain
    pub domain: String,
    // subdomain, if present
    pub subdomain: Option<String>,
    // main domain
    pub root: String,
    // tld extension
    pub tld: String,
    pub extension: String,
}

#[derive(Debug, Clone)]
pub struct DomainCast {
    pub utf8: String,
    pub ascii: String,
}

#[derive(Debug, Clone)]
pub struct DomainDetail {
    pub icann: bool,
    pub local: bool,
    pub private: bool,
    pub ascii: DomainSpec,
    pub utf8: DomainSpec,
}

pub const LOCAL_EXTENSIONS: &[&str] = &[
    "localhost",
    "localdomain",
    "arpa",
    "test",
    "invalid",
    "example",
    "internal",
];

/// Holds information about a particular domain name
impl Domain {
    /// Converts an Internationalized Domain Name (IDN) to its ASCII-compatible encoding.
    ///
    /// # Description
    /// This function takes an input that can be referenced as a string, trims any leading
    /// or trailing whitespace, and converts it to its ASCII-compatible representation
    /// using the `domain_to_ascii` utility. If the input is empty after trimming, it returns
    /// an empty string.
    ///
    /// # Type Parameters
    /// - `T`: A type that implements the `AsRef<str>` trait, allowing for flexible input handling.
    ///
    /// # Parameters
    /// - `input`: The input value that represents the IDN to be converted. It can be any type
    ///   that implements the `AsRef<str>` trait, such as `String` or `&str`.
    ///
    /// # Returns
    /// A `String` containing the ASCII-compatible encoding of the provided IDN. If the input
    /// is empty after trimming, an empty string is returned.
    ///
    /// # Example
    /// ```rust
    /// let ascii_domain = idn_to_ascii(" 例え.テスト ");
    /// assert_eq!(ascii_domain, "xn--r8jz45g.xn--zckzah");
    /// ```
    ///
    /// # Panics
    /// This function does not explicitly panic due to input constraints, but it assumes that
    /// `domain_to_ascii` correctly handles or validates conversion errors.
    ///
    /// # Notes
    /// - Whitespace around the input is removed before processing.
    /// - Ensure the `domain_to_ascii` function is available and correctly implemented
    ///   in your project for the IDN conversion to ASCII.
    pub fn idn_to_ascii<T: AsRef<str>>(input: T) -> String {
        let raw_input = input.as_ref().trim();
        if raw_input.is_empty() {
            return "".to_string();
        }
        domain_to_ascii(raw_input)
    }

    /// Converts an Internationalized Domain Name (IDN) from its ASCII-compatible encoding (ACE)
    /// to its Unicode representation.
    ///
    /// # Parameters
    /// - `input`: A value that can be referenced as a string slice (`&str`), representing the domain name in ACE format.
    ///
    /// # Returns
    /// A `String` containing the Unicode representation of the domain name. If the input is empty or consists only of
    /// whitespace, the function returns an empty string.
    ///
    /// # Examples
    /// ```
    /// let unicode_domain = idn_to_unicode("xn--bcher-kva.ch");
    /// assert_eq!(unicode_domain, "bücher.ch");
    ///
    /// let empty = idn_to_unicode("");
    /// assert_eq!(empty, "");
    /// ```
    ///
    /// # Notes
    /// - The function internally trims leading and trailing whitespace from the input string before processing.
    /// - This function relies on a helper function `domain_to_unicode`, which performs the actual IDN to Unicode conversion.
    /// Ensure that this helper function is available in the codebase.
    pub fn idn_to_unicode<T: AsRef<str>>(input: T) -> String {
        let raw_input = input.as_ref().trim();
        if raw_input.is_empty() {
            return "".to_string();
        }
        domain_to_unicode(raw_input)
    }

    /// Parses a given domain name input and returns its detailed representation, including ASCII and UTF-8 formats.
    ///
    /// # Generic Types
    /// - `T`: Any type that implements the `AsRef<str>` trait, ensuring the input can be treated as a string slice.
    ///
    /// # Arguments
    /// - `input`: The domain name input to be parsed. It can be passed as a variety of string-like types.
    ///
    /// # Returns
    /// - `Ok(DomainDetail)`: If the domain name is successfully parsed into its detailed structure.
    /// - `Err(Error)`: If the domain name is invalid or doesn't meet the necessary criteria.
    ///
    /// # Errors
    /// Returns an error in the following cases:
    /// - If the input starts with a '.' character.
    /// - If the input does not contain a '.' character.
    /// - If the input contains invalid domain characters or exceeds the maximum domain length (253 characters).
    /// - If the top-level domain (TLD) in the input does not have a valid format.
    /// - If the Unicode conversion of the domain fails.
    ///
    /// # Behavior
    /// 1. **Fast Path for "localhost":**
    ///    - If the input domain is "localhost" (case-insensitive), it is immediately recognized and returned as a local domain.
    /// 2. **Validation:**
    ///    - Ensures the domain starts with the appropriate character(s) and contains at least one dot (`.`).
    ///    - Converts the domain to its ASCII equivalent and validates its structure and length.
    ///    - Verifies the TLD against a defined regex pattern.
    /// 3. **Parsing:**
    ///    - Uses a domain name parsing library to extract specific components (e.g., subdomain, root, suffix).
    /// 4. **UTF-8 Conversion:**
    ///    - Converts the domain to its Unicode (UTF-8) representation and maps corresponding components (e.g., root, TLD, and subdomain).
    ///
    /// # Components of `DomainDetail`
    /// The parsed domain details include the following properties:
    /// - `icann`: A boolean indicating whether the domain is an ICANN-recognized domain.
    /// - `local`: A boolean indicating whether the domain is local (e.g., "localhost" or matches certain local extensions).
    /// - `ascii`: A `DomainSpec` containing the ASCII representation of the domain, including its subdomain, root, TLD, and extension.
    /// - `utf8`: A `DomainSpec` containing the Unicode (UTF-8) representation of the domain.
    ///
    /// # Example
    /// ```rust
    /// let domain = "example.com";
    /// match parse(domain) {
    ///     Ok(details) => {
    ///         println!("ASCII Domain: {}", details.ascii.domain);
    ///         println!("UTF-8 Domain: {}", details.utf8.domain);
    ///     }
    ///     Err(e) => eprintln!("Invalid domain: {}", e),
    /// }
    /// ```
    ///
    /// # Dependencies
    /// This function relies on external libraries for domain name processing:
    /// - `addr`: Parses the domain name into structured components.
    /// - `TLD_REGEX`: A pre-defined regex pattern for validating top-level domains.
    /// - `LOCAL_EXTENSIONS`: A set of predefined local domain extensions for determining local domains.
    ///
    /// # Notes
    /// - The function assumes the maximum domain name length to be in compliance with the ICANN specification (253 characters).
    /// - Unicode representation errors are handled gracefully, ensuring invalid inputs are appropriately rejected.
    pub fn parse<T: AsRef<str>>(input: T) -> ResultError<DomainDetail> {
        let raw_input = input.as_ref().trim();
        if raw_input.is_empty() {
            return Err(Error::invalid_input("Domain cannot be empty"));
        }
        // 1. Fast path for localhost
        if raw_input.eq_ignore_ascii_case("localhost") {
            let spec = DomainSpec {
                domain: "localhost".to_string(),
                subdomain: None,
                root: "localhost".to_string(),
                tld: "".to_string(),
                extension: "".to_string(),
            };
            return Ok(DomainDetail {
                icann: false,
                private: true,
                local: true,
                ascii: spec.clone(),
                utf8: spec,
            });
        }
        // if started with dot
        if raw_input.starts_with('.') {
            return Err(Error::invalid_input("Domain must not start with dot"));
        }

        if !raw_input.contains('.') {
            return Err(Error::invalid_input("Domain must contain a dot"));
        }
        let ascii_domain = domain_to_ascii(raw_input);
        if ascii_domain.is_empty() {
            return Err(Error::invalid_input("Invalid domain characters or length"));
        }

        if ascii_domain.len() > 253 {
            return Err(Error::invalid_range("Domain name too long"));
        }
        let ascii_clone = ascii_domain.to_string();
        let parsed = PSL::parse_domain_name(&ascii_domain)?;
        let ascii_tld = ascii_domain.split('.').last().unwrap_or("");
        if !TLD_REGEX.is_match(ascii_tld) {
            return Err(Error::invalid_input("Invalid TLD format"));
        }
        let ascii_extension = if parsed.has_known_suffix() {
            parsed.suffix().to_string()
        } else {
            ascii_tld.to_string()
        };

        let ascii_root = parsed.root().unwrap_or("").to_string();
        let ascii_subdomain = parsed.prefix().map(|s| s.to_string());
        let utf8_full = domain_to_unicode(&ascii_domain);
        if utf8_full.is_empty() {
            return Err(Error::invalid_input("Failed to produce valid Unicode"));
        }
        let utf8_parts: Vec<&str> = utf8_full.split('.').collect();
        let ascii_root_labels = if ascii_root.is_empty() {
            0
        } else {
            ascii_root.split('.').count()
        };
        let ascii_ext_labels = ascii_extension.split('.').count();

        let utf8_root = if ascii_root_labels > 0 && utf8_parts.len() >= ascii_root_labels {
            utf8_parts[utf8_parts.len() - ascii_root_labels..].join(".")
        } else {
            String::new()
        };

        let utf8_extension = if utf8_parts.len() >= ascii_ext_labels {
            utf8_parts[utf8_parts.len() - ascii_ext_labels..].join(".")
        } else {
            utf8_parts.last().unwrap_or(&"").to_string()
        };

        let utf8_tld = utf8_parts.last().unwrap_or(&"").to_string();

        // Subdomain UTF8 logic
        let utf8_subdomain = if !utf8_root.is_empty() && utf8_full.len() > utf8_root.len() {
            let sub = utf8_full.trim_end_matches(&utf8_root).trim_end_matches('.');
            if sub.is_empty() {
                None
            } else {
                Some(sub.to_string())
            }
        } else {
            None
        };

        Ok(DomainDetail {
            icann: parsed.is_icann(),
            local: LOCAL_EXTENSIONS.contains(&ascii_tld),
            private: parsed.is_private(),
            ascii: DomainSpec {
                domain: ascii_clone,
                subdomain: ascii_subdomain,
                root: ascii_root,
                tld: ascii_tld.to_string(),
                extension: ascii_extension,
            },
            utf8: DomainSpec {
                domain: utf8_full,
                subdomain: utf8_subdomain,
                root: utf8_root,
                tld: utf8_tld,
                extension: utf8_extension,
            },
        })
    }

    /// Parses a domain name input into its ASCII and Unicode representations.
    ///
    /// # Generic Parameters
    /// - `T`: A type that implements the `AsRef<str>` trait, allowing the function to take
    ///   various types of string-like inputs (e.g., `&str`, `String`).
    ///
    /// # Arguments
    /// - `input`: The input domain name that will be parsed and validated.
    ///
    /// # Returns
    /// - `Ok(DomainCast)`: If the input is successfully parsed, the function returns a `DomainCast` struct
    ///   containing both the ASCII and Unicode representations of the domain name.
    /// - `Err(Error)`: If the input domain name is invalid, the function returns an error providing details about
    ///   the failure.
    ///
    /// # Errors
    /// - `Error::invalid_input`:
    ///   - If the input domain is empty after trimming.
    ///   - If the input contains invalid characters or cannot be successfully represented as a domain.
    ///   - If converting the input to Unicode fails.
    /// - `Error::invalid_range`: If the input domain name exceeds the maximum allowed length of 253 characters.
    ///
    /// # Special Cases
    /// - The string `"localhost"` (case-insensitive) is treated as a special case, and its
    ///   ASCII and Unicode representations are both set to `"localhost"`.
    ///
    /// # Examples
    ///
    /// ```rust
    /// # use your_module::{DomainCast, parse_only, Error};
    /// let result = parse_only("example.com");
    /// assert!(result.is_ok());
    /// let domain = result.unwrap();
    /// assert_eq!(domain.utf8, "example.com");
    /// assert_eq!(domain.ascii, "example.com");
    ///
    /// let invalid_result = parse_only("!!invalid domain!!");
    /// assert!(invalid_result.is_err());
    /// ```
    ///
    /// # Notes
    /// - The function internally converts the input to lowercase before further processing.
    /// - The `domain_to_ascii` and `domain_to_unicode` helper functions are applied to generate
    ///   the ASCII and Unicode versions of the input domain. These functions perform additional internal validation.
    ///
    /// # Structs Used
    /// - `DomainCast`: A struct representing both the ASCII and Unicode versions of a domain name, with fields:
    ///   - `utf8`: A `String` holding the Unicode representation.
    ///   - `ascii`: A `String` holding the ASCII representation.
    ///
    /// # Constraints
    /// - The input domain name must conform to standard domain name rules (e.g., valid characters and length).
    pub fn parse_only<T: AsRef<str>>(input: T) -> ResultError<DomainCast> {
        let raw_input = input.as_ref().trim();
        if raw_input.is_empty() {
            return Err(Error::invalid_input("Domain cannot be empty"));
        }
        if raw_input.eq_ignore_ascii_case("localhost") {
            return Ok(DomainCast {
                utf8: "localhost".to_string(),
                ascii: "localhost".to_string(),
            });
        }
        let raw_input = raw_input.to_lowercase();
        let ascii_domain = domain_to_ascii(&raw_input);
        if ascii_domain.is_empty() {
            return Err(Error::invalid_input("Invalid domain characters or length"));
        }
        if ascii_domain.len() > 253 {
            return Err(Error::invalid_range("Domain name too long"));
        }
        let utf8_full = domain_to_unicode(&ascii_domain);
        if utf8_full.is_empty() {
            return Err(Error::invalid_input("Failed to produce valid Unicode"));
        }
        Ok(DomainCast {
            utf8: utf8_full,
            ascii: ascii_domain,
        })
    }

    pub fn parse_from_url<T: Into<String>>(input: T) -> ResultError<DomainDetail> {
        let mut raw_input = input.into().trim().to_lowercase();
        if raw_input.starts_with("//") {
            raw_input = format!("https:{}", raw_input);
        } else if !raw_input.contains("://") && !PROTOCOL_REGEX.is_match(raw_input.as_str()) {
            raw_input = format!("https://{}", raw_input);
        }
        let uri = Url::parse(raw_input.as_str())
            .map_err(|_| Error::invalid_input("Invalid URL format"))?;
        if let Some(host) = uri.host_str() {
            return Self::parse(host);
        }
        Err(Error::invalid_input("Invalid URL format"))
    }

    pub fn parse_email<T: AsRef<str>>(email: T) -> ResultError<String> {
        let email = email.as_ref();
        if !email.contains('@') {
            return Err(Error::invalid_input(format!(
                "Email {} does not valid",
                email
            )));
        }
        let mut split: Vec<String> = email.split("@").map(|e| e.to_string()).collect();
        if split.len() > 2 {
            return Err(Error::invalid_input(format!(
                "Email {} does not valid! too many separator @",
                email
            )));
        }
        let domain = split
            .pop()
            .ok_or(Error::invalid_input("Domain name is empty"))?;
        let address = split
            .pop()
            .ok_or(Error::invalid_input("Email Address is empty"))?;
        if domain.is_empty() {
            return Err(Error::invalid_input("Domain name is empty"));
        }
        let (local, _comments) = Self::parse_local_part_mail(&address)?;
        let domain = Self::parse_only(domain)?;

        // address
        Ok(format!("{}@{}", local, domain.ascii))
    }

    pub fn parse_local_part_mail(address: &str) -> ResultError<(String, Vec<String>)> {
        let len = address.len();
        if len == 0 {
            return Err(Error::invalid_input("Email Address is empty"));
        }
        if len > 64 {
            return Err(Error::invalid_input("Email address too long"));
        }
        if address.starts_with('.') || address.ends_with('.') {
            return Err(Error::invalid_input(
                "Email address can not start or end with dot (.)",
            ));
        }
        let mut comments = Vec::new();
        let mut result = String::new();
        let mut inside_quotes = false;
        let mut comment_depth = 0;
        let mut escaped = false;
        let mut prev = ' ';
        let special = "!#$%&'*+-/=?^_`{|}~()<>@,;:\"\\[]";
        let alpha_numeric = "abcdefghijklmnopqrstuvwxyzABCDEFGHIJKLMNOPQRSTUVWXYZ01234567890";
        let quoted_chars = "()<>@,;:\"\\[] ";
        let chars = quoted_chars.chars().collect::<Vec<char>>();
        // RFC: CTLs (0-31 & 127) + SPACE
        let ctl_s: String = (0..32u8).chain(std::iter::once(127u8)).map(|b| b as char).collect();
        let space = " ";
        let allowed = format!("{}{}{}{}{}", special, alpha_numeric, quoted_chars, ctl_s, space)
            .chars()
            .collect::<Vec<char>>();

        let first_char = address.chars().next().unwrap();
        if first_char != '(' && first_char != '"' && !alpha_numeric.contains(first_char) {
            return Err(Error::invalid_input(
                "Email address should start with alpha numeric character",
            ));
        }
        if len > 1 {
            let last_char = address.chars().last().unwrap();
            if last_char != ')' && last_char != '"' && !alpha_numeric.contains(last_char) {
                return Err(Error::invalid_input(
                    "Email address should end with alpha numeric character",
                ));
            }
        }

        let mut last_comment = String::new();

        for c in address.chars() {
            if c != '"' && !allowed.contains(&c) {
                return Err(Error::invalid_input(format!(
                    "Character {} is not allowed",
                    c
                )));
            }

            if escaped {
                if comment_depth == 0 {
                    result.push(c);
                } else {
                    last_comment.push(c);
                }
                escaped = false;
                prev = c;
                continue;
            }

            match c {
                '\\' => {
                    escaped = true;
                    if comment_depth == 0 {
                        result.push(c);
                    } else {
                        last_comment.push(c);
                    }
                }
                '"' if comment_depth == 0 => {
                    inside_quotes = !inside_quotes;
                    result.push(c);
                }
                '(' if !inside_quotes => {
                    if comment_depth == 0 {
                        last_comment = String::new();
                    } else {
                        last_comment.push(c);
                    }
                    comment_depth += 1;
                }
                ')' if !inside_quotes => {
                    if comment_depth == 0 {
                        return Err(Error::invalid_input("Invalid bracket parenthesis"));
                    }
                    comment_depth -= 1;
                    if comment_depth == 0 {
                        comments.push(last_comment.clone());
                    } else {
                        last_comment.push(c);
                    }
                }
                _ if comment_depth == 0 => {
                    if !inside_quotes {
                        if c == '.' && prev == '.' {
                            return Err(Error::invalid_input("Double dot is not allowed"));
                        }
                        if c != '.' && chars.contains(&c) {
                            return Err(Error::invalid_input(format!(
                                "Special character '{}' must be inside quote!",
                                c
                            )));
                        }
                    }
                    result.push(c);
                }
                _ => {
                    last_comment.push(c);
                }
            }

            if comment_depth == 0 {
                prev = c;
            }
        }

        if inside_quotes {
            return Err(Error::invalid_input("Quote not closed"));
        }

        if comment_depth > 0 {
            return Err(Error::invalid_input("Comment not closed"));
        }

        Ok((result, comments))
    }
}
