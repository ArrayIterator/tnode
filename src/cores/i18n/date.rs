use chrono::{DateTime, FixedOffset, TimeZone, Utc};
use log::warn;
use serde::Deserialize;
use std::collections::HashMap;
use std::sync::OnceLock;
use std::time::{SystemTime, UNIX_EPOCH};
static DATE_DATA: OnceLock<HashMap<String, DateData>> = OnceLock::new();

/// {
//     "Australia/Adelaide": {
//         "country_name": "Australia",
//         "country_codes": "AU",
//         "latitude": -34.91667,
//         "longitude": 138.58333,
//         "zone_name": "Australia/Adelaide",
//         "abbreviation": "CAST",
//         "offset": 34200,
//         "diff": {
//             "hours": 9,
//             "minutes": 30,
//             "seconds": 0
//         },
//         "dst": false,
//         "comments": "South Australia"
//     }
// }
// ISO8601
///

/// A struct representing the time difference details.
/// # Fields
///
/// * `hours` (`i32`) - The number of hours in the time difference.
/// * `minutes` (`i32`) - The number of minutes in the time difference.
/// * `seconds` (`i32`) - The number of seconds in the time difference.
///
#[derive(Deserialize, Debug, Clone)]
pub struct Diff {
    pub hours: i32,
    pub minutes: i32,
    pub seconds: i32,
}

/// Represents data related to a specific geographic location and its time zone.
///
/// # Fields
///
/// * `country_name` (`String`) - The name of the country associated with the time zone.
/// * `country_codes` (`String`) - The country code(s) associated with the time zone (ISO 3166-1 alpha-2 or similar).
/// * `latitude` (`f64`) - The latitude of the geographic location.
/// * `longitude` (`f64`) - The longitude of the geographic location.
/// * `zone_name` (`String`) - The name of the time zone (e.g., "America/New_York").
/// * `abbreviation` (`String`) - The time zone abbreviation (e.g., "EST", "PST").
/// * `offset` (`i32`) - The current UTC offset in seconds for the time zone.
/// * `diff` (`Diff`) - A custom structure representing the time difference details.
/// * `dst` (`bool`) - Indicates whether Daylight Saving Time (DST) is currently in effect for the time zone.
/// * `comments` (`String`) - Additional comments or notes regarding the time zone or location.
#[derive(Deserialize, Debug, Clone)]
pub struct DateData {
    pub country_name: String,
    pub country_codes: String,
    pub latitude: f64,
    pub longitude: f64,
    pub zone_name: String,
    pub abbreviation: String,
    pub offset: i32,
    pub diff: Diff,
    pub dst: bool,
    pub comments: String,
}

/// A struct representing a `Date`.
///
/// The `Date` struct is a simple representation of a date. It derives the
/// `Debug` trait to enable formatting for debugging purposes and the `Clone`
/// trait to allow for duplication of `Date` instances.
///
/// # Examples
///
/// ```
/// #[derive(Debug, Clone)]
/// pub struct Date;
///
/// let date = Date;
/// let cloned_date = date.clone();
/// println!("{:?}", cloned_date);
/// ```
///
/// Note: This struct currently does not contain any fields or methods. Further
/// functionality can be added as needed.
///
/// Visibility: This struct is scoped to the current crate (`pub(crate)`).
#[derive(Debug, Clone)]
pub struct Date {
    date_data: DateData,
    pub nano: Option<i64>,
}

fn micro_to_nano(micro: i64) -> i64 {
    micro * 1_000
}
fn millis_to_nano(millis: i64) -> i64 {
    millis * 1_000_000
}
fn unix_to_nano(timestamp: i64) -> i64 {
    timestamp * 1_000_000_000
}

/// Date struct implements methods to get timezone data by zone name or country name.
/// This data is based on the ISO 8601 standard.
impl Date {
    // Standard ISO 8601 / RFC 3339
    const ATOM: &'static str = "%Y-%m-%dT%H:%M:%SZ";
    const RFC3339: &'static str = "%Y-%m-%dT%H:%M:%S%+:z";
    const ISO8601: &'static str = "%Y-%m-%dT%H:%M:%S%+:z";

    // Email Standards
    const RFC2822: &'static str = "%a, %d %b %Y %H:%M:%S %z";
    const RFC822: &'static str = "%a, %d %b %y %H:%M:%S %z"; // RFC822 use 2 digit year (%y)

    // Browser / Cookie Standards
    const COOKIE: &'static str = "%A, %d-%b-%Y %H:%M:%S GMT";
    const RFC850: &'static str = "%A, %d-%b-%y %H:%M:%S GMT";

    // Web / Database Standards
    const W3C: &'static str = "%Y-%m-%dT%H:%M:%S%.f%+:z";
    // SQL Format
    const ISO_9075: &'static str = "%Y-%m-%d %H:%M:%S";

    /// Creates a new instance of the struct.
    ///
    /// # Parameters
    /// - `date`: A `DateData` object that represents the date to initialize the struct with.
    ///
    /// # Returns
    /// A new instance of the struct with the provided `date` and `nano` set to `None`.
    ///
    /// # Example
    /// ```
    /// let date = DateData::new(2023, 10, 5);
    /// let instance = MyStruct::new(date);
    /// ```
    pub fn new(date: DateData) -> Self {
        Self {
            date_data: date,
            nano: None,
        }
    }

    /// Creates a new instance of the struct from a given nanosecond value and date data.
    ///
    /// # Parameters
    /// - `nano`: The nanosecond value represented as a 64-bit integer (`i64`).
    /// - `date`: The date information encapsulated in a `DateData` object.
    ///
    /// # Returns
    /// A new instance of the struct containing the specified `nano` and `date_data` values.
    ///
    /// # Example
    /// ```rust
    /// let date = DateData::new(2023, 10, 5); // Example DateData initialization
    /// let instance = YourStruct::from_nano(1_000_000_000, date);
    /// ```
    ///
    /// In this example, a new `YourStruct` instance is created with the specified nanosecond value
    /// (`1_000_000_000`) and `DateData` object.
    ///
    /// # Notes
    /// - The `nano` field is wrapped in an `Option`, indicating that it can be `Some(value)` or `None`.
    /// - Ensure the `DateData` structure is valid and properly initialized.
    ///
    pub fn from_nano(nano: i64, date: DateData) -> Self {
        Self {
            date_data: date,
            nano: Some(nano),
        }
    }

    /// Creates an instance of `Self` from a given Unix timestamp and additional date metadata.
    ///
    /// This function converts a Unix timestamp (seconds since the Unix epoch,
    /// i.e., January 1, 1970) into a nanosecond-based timestamp and then
    /// combines it with the provided `DateData` to construct the desired object.
    ///
    /// # Arguments
    ///
    /// * `timestamp` - An `i64` value representing the Unix timestamp in seconds.
    /// * `date` - A `DateData` structure containing additional date-related metadata.
    ///
    /// # Returns
    ///
    /// * Returns an instance of `Self` constructed from the provided Unix timestamp and date metadata.
    ///
    /// # Example
    ///
    /// ```rust
    /// let unix_timestamp: i64 = 1672531200; // Example Unix timestamp
    /// let date_metadata = DateData::new();  // Assume this creates a valid DateData instance
    /// let instance = YourStruct::from_unix_timestamp(unix_timestamp, date_metadata);
    /// ```
    pub fn from_unix_timestamp(timestamp: i64, date: DateData) -> Self {
        Self::from_nano(unix_to_nano(timestamp), date)
    }

    /// Creates a new instance of the type from the given timestamp in milliseconds and date information.
    ///
    /// # Parameters
    ///
    /// - `millis`: The timestamp in milliseconds, represented as an `i64`.
    /// - `date`: The `DateData` representing date-specific information required to construct the instance.
    ///
    /// # Returns
    ///
    /// Returns a new instance of `Self` by converting the provided milliseconds to nanoseconds
    /// and utilizing the specified date.
    ///
    /// # Panics
    ///
    /// This function will panic if the conversion from milliseconds to nanoseconds results in
    /// an overflow or if the values provided are otherwise invalid for the `from_nano` function.
    ///
    /// # Examples
    ///
    /// ```rust
    /// let millis = 1_637_239_500_000; // Example timestamp in milliseconds
    /// let date = DateData::new(/* date-specific parameters */);
    /// let instance = CustomType::from_millis(millis, date);
    /// ```
    pub fn from_millis(millis: i64, date: DateData) -> Self {
        Self::from_nano(millis_to_nano(millis), date)
    }

    /// Creates a new instance of the struct from the given microsecond timestamp and date data.
    ///
    /// This function converts the provided microsecond timestamp into nanoseconds
    /// before delegating to the `from_nano` method to construct the instance.
    ///
    /// # Arguments
    /// * `millis` - A 64-bit integer representing the time in microseconds.
    /// * `date` - A `DateData` object containing the date information.
    ///
    /// # Returns
    /// A new instance of the struct that represents the given timestamp and date.
    ///
    /// # Examples
    /// ```
    /// let timestamp_micro = 1_000_000; // 1 second in microseconds
    /// let date_data = DateData::new(year, month, day);
    /// let instance = YourStruct::from_micro(timestamp_micro, date_data);
    /// ```
    pub fn from_micro(millis: i64, date: DateData) -> Self {
        Self::from_nano(micro_to_nano(millis), date)
    }

    /// Creates a new instance of the struct with the provided nanosecond timestamp.
    ///
    /// This method clones the existing `date_data` field from the current instance
    /// and sets the `nano` field to the given nanosecond timestamp value.
    ///
    /// # Arguments
    ///
    /// * `nano` - A 64-bit integer representing the nanosecond timestamp to be set.
    ///
    /// # Returns
    ///
    /// A new instance of the struct with the updated `nano` value and the same `date_data`
    /// as the original instance.
    ///
    /// # Example
    ///
    /// ```rust
    /// let original = MyStruct {
    ///     date_data: some_date_data,
    ///     nano: None,
    /// };
    /// let updated = original.with_nano_timestamp(1_000_000_000);
    /// assert_eq!(updated.nano, Some(1_000_000_000));
    /// ```
    pub fn with_nano_timestamp(&self, nano: i64) -> Self {
        Self {
            date_data: self.date_data.clone(),
            nano: Some(nano),
        }
    }

    /// Updates the current object with a new timestamp in microseconds precision by converting it
    /// to nanoseconds precision and reusing the `with_nano_timestamp` method.
    ///
    /// # Parameters
    /// - `timestamp` (i64): The new timestamp value in microseconds.
    ///
    /// # Returns
    /// - `Self`: A new instance of the object with the updated timestamp in nanoseconds.
    ///
    /// # Example
    /// ```rust
    /// let original = MyStruct::new();
    /// let updated = original.with_micro_timestamp(1_675_849_500_123_456); // Microseconds
    /// ```
    ///
    /// # Notes
    /// This function internally calls `with_nano_timestamp` after converting the input
    /// microsecond timestamp to nanoseconds using the `micro_to_nano` helper function.
    ///
    /// Ensure that `micro_to_nano` handles large values gracefully to avoid integer overflow.
    pub fn with_micro_timestamp(&self, timestamp: i64) -> Self {
        self.with_nano_timestamp(micro_to_nano(timestamp))
    }

    /// Updates the current instance with a timestamp provided in milliseconds.
    ///
    /// This method takes a timestamp in milliseconds as input, converts it
    /// into nanoseconds, and then calls `with_nano_timestamp` to update
    /// the timestamp of the current instance.
    ///
    /// # Parameters
    ///
    /// - `timestamp`: The timestamp in milliseconds as a signed 64-bit integer (`i64`).
    ///
    /// # Returns
    ///
    /// Returns a new instance of `Self` with the updated timestamp.
    ///
    /// # Example
    ///
    /// ```rust
    /// let instance = ExampleStruct::new();
    /// let updated_instance = instance.with_millis_timestamp(1672531200000);
    /// ```
    ///
    /// Here, `1672531200000` is a sample timestamp in milliseconds.
    ///
    /// # Notes
    ///
    /// This method assumes that the `millis_to_nano` utility function properly
    /// converts milliseconds to nanoseconds and that the `with_nano_timestamp`
    /// method will correctly handle the nanosecond value.
    pub fn with_millis_timestamp(&self, timestamp: i64) -> Self {
        self.with_nano_timestamp(millis_to_nano(timestamp))
    }

    /// Updates the current instance with a new timestamp specified in Unix timestamp format (seconds since the epoch).
    ///
    /// This function converts the given Unix timestamp (in seconds) to a corresponding nanosecond-based timestamp
    /// and applies it to the instance.
    ///
    /// # Arguments
    /// * `timestamp` - A 64-bit integer representing the Unix timestamp (seconds since January 1, 1970).
    ///
    /// # Returns
    /// * `Self` - A new instance with the updated timestamp in nanoseconds.
    ///
    /// # Example
    /// ```rust
    /// let updated_instance = instance.with_unix_timestamp(1672531200);
    /// ```
    ///
    /// # Notes
    /// This method internally calls the `unix_to_nano` function to convert the Unix timestamp
    /// to nanoseconds, and then delegates to `with_nano_timestamp` to finalize the update.
    pub fn with_unix_timestamp(&self, timestamp: i64) -> Self {
        self.with_nano_timestamp(unix_to_nano(timestamp))
    }

    /// Checks if the `nano` field in the struct is set to `Some` value.
    ///
    /// # Returns
    ///
    /// * `true` - If the `nano` field contains a value (i.e., is `Some`).
    /// * `false` - If the `nano` field is `None`.
    ///
    /// # Example
    ///
    /// ```rust
    /// let example = YourStruct { nano: Some(42) };
    /// assert!(example.is_fixed_nano());
    ///
    /// let example = YourStruct { nano: None };
    /// assert!(!example.is_fixed_nano());
    /// ```
    pub fn is_fixed_nano(&self) -> bool {
        self.nano.is_some()
    }

    /// Returns the number of nanoseconds since the UNIX epoch (January 1, 1970, 00:00:00 UTC),
    /// or a negative value for times before the epoch.
    ///
    /// # Behavior
    /// - If the `nano` field of the struct is set (`Some(nano)`), the method directly returns its value.
    /// - If the `nano` field is `None`, the method calculates the current time in nanoseconds since the
    ///   UNIX epoch based on the system clock (`SystemTime::now()`). If the current time is before the
    ///   UNIX epoch, the method returns the negative duration.
    ///
    /// # Returns
    /// - `i64`: The number of nanoseconds since the UNIX epoch as a signed 64-bit integer. A negative
    ///   value indicates a timestamp before the UNIX epoch.
    ///
    /// # Errors
    /// - The method does not explicitly return errors but handles potential errors internally by returning
    ///   a negative value in cases where the current system time is before the UNIX epoch.
    ///
    /// # Example
    /// ```
    /// let result = instance.nano();
    /// println!("Nanoseconds since epoch: {}", result);
    /// ```
    ///
    /// # Notes
    /// - The conversion of `duration.as_nanos()` to `i64` assumes the nanosecond value fits within the
    ///   range of an `i64`. Overflow could occur for extremely large durations, though this is unlikely
    ///   in practical usage scenarios.
    pub fn nano(&self) -> i64 {
        if let Some(nano) = self.nano {
            nano
        } else {
            match SystemTime::now().duration_since(UNIX_EPOCH) {
                Ok(duration) => duration.as_nanos() as i64,
                Err(e) => {
                    // If the time is before 1970,
                    // duration_since returns an Error containing the negative duration from the EPOCH.
                    -(e.duration().as_nanos() as i64)
                }
            }
        }
    }

    /// Returns a reference to the `DateData` associated with the current instance.
    ///
    /// # Examples
    ///
    /// ```
    /// let instance = SomeStruct { date_data: DateData::new() };
    /// let date_data_ref = instance.data();
    /// // Now `date_data_ref` holds a reference to the date_data of the instance.
    /// ```
    ///
    /// # Returns
    ///
    /// A reference to the `DateData` contained within the object.
    pub fn data(&self) -> &DateData {
        &self.date_data
    }

    /// Formats the timestamp stored within the object into a human-readable date and time string
    /// based on the specified format.
    ///
    /// This method uses the `chrono` crate to handle timestamp conversion and formatting. It
    /// calculates the UTC time from the internally stored nanosecond timestamp, applies a fixed
    /// offset to get the local time, and formats the resulting time according to the given format
    /// pattern.
    ///
    /// # Type Parameters
    /// - `T`: A type that can be referenced as a string slice. Typically, this would be a `String`
    ///   or a string slice (`&str`).
    ///
    /// # Arguments
    /// - `format_str`: A string slice or type that can be converted to a string slice specifying
    ///   the desired format. The format should follow the formatting rules specified
    ///   in the `chrono` crate's documentation (e.g., `"%Y-%m-%d %H:%M:%S"`).
    ///
    /// # Returns
    /// A `String` representing the formatted date and time.
    ///
    /// # Panics
    /// This function will panic if:
    /// - `Utc.timestamp_opt` fails to create a valid timestamp from the calculated seconds and
    ///   nanoseconds.
    /// - `FixedOffset::east_opt` fails to create a valid time zone offset. In such cases,
    ///   a default timestamp or offset (0) is used.
    ///
    /// # Example
    /// ```
    /// use chrono::{Utc, TimeZone, FixedOffset};
    ///
    /// let custom_time = YourStruct { /* initialization */ };
    /// let formatted_time = custom_time.format("%Y-%m-%d %H:%M:%S");
    /// println!("Formatted time: {}", formatted_time);
    /// ```
    ///
    /// Make sure to replace `YourStruct` with the actual struct that provides this method.
    ///
    /// # Notes
    /// - Internally, this method utilizes the `nano` method to retrieve a nanosecond timestamp.
    ///   Ensure the `nano` method is implemented and provides a valid 64-bit nanosecond
    ///   timestamp.
    /// - The `offset` for the local time is derived from the `date_data.offset` field. Make sure
    ///   this field is properly populated before calling this method, as it affects the calculation
    ///   of the local time.
    pub fn format<T: AsRef<str>>(&self, format_str: T) -> String {
        let nano_timestamp = self.nano();
        let secs = nano_timestamp / 1_000_000_000;
        let n_secs = (nano_timestamp % 1_000_000_000) as u32;

        let utc_time = Utc
            .timestamp_opt(secs, n_secs)
            .single()
            .unwrap_or_else(|| Utc.timestamp_nanos(0));
        let offset = FixedOffset::east_opt(self.date_data.offset)
            .unwrap_or_else(|| FixedOffset::east_opt(0).unwrap());
        let local_time = utc_time.with_timezone(&offset);
        local_time.format(format_str.as_ref()).to_string()
    }

    /// Returns the current date and time with a fixed offset applied.
    ///
    /// This function retrieves the current UTC date and time, then applies a fixed
    /// offset based on the `date_data.offset` field. If the specified offset is invalid
    /// or unavailable, a default offset of `0` (UTC) is used. The resulting `DateTime`
    /// is adjusted to the specified offset and returned.
    ///
    /// # Returns
    /// A `DateTime<FixedOffset>` instance representing the current date and time
    /// with the specified fixed offset.
    ///
    /// # Example
    /// ```
    /// let datetime = your_instance.now();
    /// println!("Current date and time with offset: {}", datetime);
    /// ```
    ///
    /// # Panics
    /// This function does not panic as it uses a fallback mechanism for the offset.
    ///
    /// # Note
    /// Ensure that `date_data.offset` contains a valid timezone offset in seconds.
    /// If invalid, a default UTC offset is applied.
    pub fn now(&self) -> DateTime<FixedOffset> {
        let offset = FixedOffset::east_opt(self.date_data.offset)
            .unwrap_or_else(|| FixedOffset::east_opt(0).unwrap());
        Utc::now().with_timezone(&offset)
    }

    /// Converts the date into an ATOM format string.
    ///
    /// The ATOM format is a standardized date and time representation
    /// defined by RFC 4287. It is commonly used in web feeds and
    /// other applications requiring precise and standardized date formats.
    ///
    /// # Returns
    ///
    /// A `String` containing the date formatted in ATOM format (RFC 3339-compliant).
    ///
    /// # Example
    ///
    /// ```
    /// let date = your_date_object.atom();
    /// println!("{}", date); // Outputs the date in ATOM format, e.g., "2023-10-05T15:23:01+00:00".
    /// ```
    ///
    /// # Notes
    ///
    /// This function internally calls `self.format(Date::ATOM)` to perform the formatting.
    pub fn atom(&self) -> String {
        //
        self.format(Date::ATOM)
    }

    /// Converts the `Date` object into a string formatted in RFC 3339 standard.
    ///
    /// # Returns
    ///
    /// A `String` representing the date and time in RFC 3339 format, which is
    /// a widely-used date/time format in applications such as web APIs and logging.
    ///
    /// # Examples
    ///
    /// ```
    /// let date = Date::now();
    /// let formatted_date = date.rfc3339();
    /// println!("RFC 3339 formatted date: {}", formatted_date);
    /// ```
    ///
    /// This method internally uses the `Date::RFC3339` constant as the format specifier
    /// to provide the correct output in the expected standard format.
    ///
    /// # Notes
    ///
    /// - Ensure the `Date` object is properly initialized before calling this method.
    /// - The output string follows the format: `YYYY-MM-DDTHH:MM:SSZ` or `YYYY-MM-DDTHH:MM:SS[+-]HH:MM`
    ///   depending on whether the timestamp is in UTC or a specific timezone.
    pub fn rfc3339(&self) -> String {
        self.format(Date::RFC3339)
    }

    /// Formats the date instance into an ISO 8601 string representation.
    ///
    /// # Returns
    ///
    /// A `String` containing the date formatted in ISO 8601 standard.
    ///
    /// # Example
    ///
    /// ```rust
    /// let date = Date::new(2023, 10, 15); // Example of constructing a date.
    /// let iso_string = date.iso8601();
    /// assert_eq!(iso_string, "2023-10-15");
    /// ```
    ///
    /// # Notes
    ///
    /// This method uses the `Date::ISO8601` format specification to produce the output.
    ///
    /// # See Also
    ///
    /// - [`Date::format`] for general formatting with custom patterns.
    pub fn iso8601(&self) -> String {
        self.format(Date::ISO8601)
    }

    /// Formats the date according to the RFC 2822 standard.
    ///
    /// This method generates a string representation of the date
    /// in the RFC 2822 format, which is commonly used in email headers
    /// and is specified by the Internet Message Format standard.
    ///
    /// # Returns
    ///
    /// * A `String` representing the date in RFC 2822 format.
    ///
    /// # Example
    ///
    /// ```rust
    /// let date = Date::now();
    /// let formatted_date = date.rfc2822();
    /// println!("{}", formatted_date); // Example output: "Mon, 25 Sep 2023 14:38:00 +0000"
    /// ```
    pub fn rfc2822(&self) -> String {
        self.format(Date::RFC2822)
    }

    /// Formats the date instance into an RFC822-compliant string.
    ///
    /// # Returns
    /// A `String` representing the date and time in the RFC822 format.
    ///
    /// RFC822 is a standard for date and time representation used in email
    /// and internet message headers. The format includes the day of the week,
    /// day of the month, month, year, time, and time zone.
    ///
    /// # Example
    /// ```
    /// let date = Date::now();
    /// let formatted_date = date.rfc822();
    /// println!("{}", formatted_date); // "Mon, 20 Feb 2023 14:28:00 +0000"
    /// ```
    ///
    /// This function delegates its functionality to the `format` method
    /// with `Date::RFC822` as the input format.
    pub fn rfc822(&self) -> String {
        self.format(Date::RFC822)
    }

    /// Formats the date into the HTTP cookie standard format.
    ///
    /// # Returns
    ///
    /// A `String` containing the date formatted according to the
    /// [COOKIE](https://datatracker.ietf.org/doc/html/rfc6265) standard.
    ///
    /// # Example
    ///
    /// ```rust
    /// let date = Date::new();
    /// let cookie_formatted_date = date.cookie();
    /// println!("{}", cookie_formatted_date); // Example: "Tuesday, 08-Nov-2023 14:29:00 GMT"
    /// ```
    ///
    /// # Note
    ///
    /// The `COOKIE` format is commonly used in HTTP headers to set expiration dates for cookies.
    ///
    /// # See Also
    ///
    /// - `Date::format(format: &str)`: Converts the date into the specified string format.
    pub fn cookie(&self) -> String {
        self.format(Date::COOKIE)
    }

    /// Formats the date using the RFC 850 standard format.
    ///
    /// # RFC 850 Format
    /// The RFC 850 standard represents a date in the following format:
    /// `Weekday, DD-Month-YY HH:MM:SS GMT`.
    ///
    /// # Example
    /// ```
    /// let date = your_date_object;
    /// let formatted_date = date.rfc850();
    /// println!("{}", formatted_date); // Example output: "Sunday, 06-Nov-94 08:49:37 GMT"
    /// ```
    ///
    /// # Returns
    /// A `String` containing the date formatted according to the RFC 850 standard.
    ///
    /// # Notes
    /// - This function internally uses the `self.format` method with the `Date::RFC850` format specifier.
    pub fn rfc850(&self) -> String {
        self.format(Date::RFC850)
    }

    /// Generates a W3C (World Wide Web Consortium) formatted date and time string.
    ///
    /// This method converts the internal date representation into a string
    /// formatted according to the W3C standard. The W3C format is commonly
    /// used in web technologies such as RSS feeds, APIs, and other internet-based
    /// systems requiring a standardized date-time format.
    ///
    /// # Returns
    /// * `String` - A string representation of the date and time in W3C format.
    ///
    /// # Example
    /// ```rust
    /// let date = your_date_object.w3c();
    /// println!("{}", date); // Example output: "2023-03-10T12:34:56Z"
    /// ```
    pub fn w3c(&self) -> String {
        self.format(Date::W3C)
    }

    /// Formats the current date/time into the ISO 9075 standard format.
    ///
    /// ISO 9075 is part of the SQL standard used to represent date and time values.
    /// This method internally uses the `Date::ISO_9075` format to produce a `String`
    /// representation of the date/time.
    ///
    /// # Returns
    ///
    /// A `String` containing the date/time formatted according to the ISO 9075 standard.
    ///
    /// # Examples
    ///
    /// ```rust
    /// let date_time = DateTime::now();
    /// let formatted = date_time.iso_9075();
    /// println!("{}", formatted); // Example output: "2023-10-05 14:30:00"
    /// ```
    pub fn iso_9075(&self) -> String {
        self.format(Date::ISO_9075)
    }

    /// Get all timezone data
    /// # Example
    /// ```rust
    /// use crate::cores::i18n::date::Date;
    /// /// #[tokio::main]
    /// async fn main() {
    ///     let all_data = Date::all();
    ///     println!("Total timezones: {}", all_data.len());
    /// }
    /// ```
    pub fn all() -> &'static HashMap<String, DateData> {
        DATE_DATA.get_or_init(|| {
            let json_str = include_str!("../../../resources/i18n/ISO8601.json");
            let raw_data: HashMap<String, DateData> = serde_json::from_str(json_str)
                .unwrap_or_else(|e| {
                    warn!(target: "i18n", "Error parsing ISO8601.json: {}", e);
                    HashMap::new()
                });
            raw_data
                .into_iter()
                .map(|(k, v)| (k.to_lowercase(), v))
                // filter country code to uppercase
                .map(|(k, mut v)| {
                    v.country_codes = v.country_codes.to_uppercase();
                    (k, v)
                })
                .collect()
        })
    }

    /// Get timezone data by zone name (case-insensitive)
    /// # Example
    /// ```rust
    /// use crate::cores::i18n::date::Date;
    /// /// #[tokio::main]
    /// async fn main() {
    ///     if let Some(data) = Date::get("Australia/Adelaide") {
    ///         println!("Country: {}", data.country_name);
    ///     }
    /// }
    /// ```
    pub fn get<T: AsRef<str>>(zone_name: T) -> Option<&'static DateData> {
        Self::all().get(&zone_name.as_ref().to_lowercase())
    }

    /// Find timezone data by country name (exact match, case-insensitive)
    /// # Example
    /// ```rust
    /// use crate::cores::i18n::date::Date;
    /// /// #[tokio::main]
    /// async fn main() {
    ///     let australia_timezones = Date::find_by_country("australia");
    ///     for tz in australia_timezones {
    ///         println!("Zone Name: {}", tz.zone_name);
    ///     }
    /// }
    /// ```
    pub fn find_by_country<T: AsRef<str>>(country: T) -> Vec<&'static DateData> {
        let country_lower = country.as_ref().to_lowercase();
        Self::all()
            .values()
            .filter(|data| data.country_name.to_lowercase() == country_lower)
            .collect()
    }

    /// Find timezone data by country code (exact match, case-insensitive)
    /// # Example
    /// ```rust
    /// use crate::cores::i18n::date::Date;
    /// /// #[tokio::main]
    /// async fn main() {
    ///     let au_timezones = Date::find_by_country_code("au");
    ///     for tz in au_timezones {
    ///         println!("Zone Name: {}", tz.zone_name);
    ///     }
    /// }
    /// ```
    pub fn find_by_country_code<T: AsRef<str>>(code: T) -> Vec<&'static DateData> {
        let code_upper = code.as_ref().to_uppercase();
        Self::all()
            .values()
            .filter(|data| data.country_codes.to_uppercase() == code_upper)
            .collect()
    }
}
