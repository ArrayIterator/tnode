use log::LevelFilter;

pub struct LogLevel;

impl LogLevel {
    /// ```
    /// Converts an input value into a `LevelFilter` based on its string representation.
    ///
    /// # Type Parameters
    /// - `T`: A type that can be converted into a `String`. This allows for a flexible input type,
    ///        such as `&str`, `String`, or other types implementing the `Into<String>` trait.
    ///
    /// # Parameters
    /// - `level`: The input value representing the desired log level. This is converted into a `String`
    ///            and matched against predefined log levels ("debug", "info", "warn", "error", "trace").
    ///
    /// # Returns
    /// A `LevelFilter` enum variant corresponding to the provided input. If the input doesn't match any
    /// predefined log level, it defaults to `LevelFilter::Off`.
    ///
    /// # Log Level Mapping:
    /// - `"debug"` → `LevelFilter::Debug`
    /// - `"info"`  → `LevelFilter::Info`
    /// - `"warn"`  → `LevelFilter::Warn`
    /// - `"error"` → `LevelFilter::Error`
    /// - `"trace"` → `LevelFilter::Trace`
    /// - Any other value → `LevelFilter::Off`
    ///
    /// # Examples
    /// ```
    /// use crate::LevelFilter;
    ///
    /// let filter = into("debug");
    /// assert_eq!(filter, LevelFilter::Debug);
    ///
    /// let filter = into("error");
    /// assert_eq!(filter, LevelFilter::Error);
    ///
    /// let filter = into("unknown");
    /// assert_eq!(filter, LevelFilter::Off);
    /// ```
    /// ```
    pub fn level<T: AsRef<str>>(level: T) -> LevelFilter
    {
        Self::level_filter_default(level, LevelFilter::Off)
    }

    /// Checks if the provided log level is valid.
    ///
    /// This function evaluates the input string to determine if it matches
    /// one of the predefined valid log levels: "debug", "info", "warn",
    /// "warning", "error", or "trace".
    ///
    /// # Parameters
    /// - `level`: A string slice that holds the log level to check.
    ///
    /// # Returns
    /// - `true` if the input `level` matches one of the valid log levels.
    /// - `false` otherwise.
    ///
    /// # Examples
    /// ```
    /// assert!(valid_level("info"));  // Valid log level
    /// assert!(valid_level("warn"));  // Valid log level
    /// assert!(valid_level("trace")); // Valid log level
    /// assert!(!valid_level("verbose")); // Invalid log level
    /// assert!(!valid_level(""));       // Invalid log level
    /// ```
    pub fn is_valid_level<T: AsRef<str>>(level: T) -> bool
    {
        match level.as_ref() {
            "debug" | "info" | "warn" | "warning" | "error" | "trace" => true,
            _ => false,
        }
    }

    /// Filters and normalizes the given log level string into one of the standard log level categories.
    ///
    /// This function takes a string slice representing a log level and normalizes it
    /// into one of the standard log levels: "error", "warn", "info", "debug", "trace", or "off".
    /// If the input string matches specific keywords associated with these levels, it is
    /// mapped to the corresponding standard log level. If no match is found or the input
    /// is empty, the default log level "info" is returned.
    ///
    /// # Parameters
    ///
    /// - `&self`: The reference to the instance of the type implementing this function.
    /// - `level: &str`: The input string representing the log level to be filtered and normalized.
    ///
    /// # Returns
    ///
    /// - `&str`: The normalized log level string. Possible values are "error", "warn", "info",
    ///   "debug", "trace", or "off".
    ///
    /// # Examples
    ///
    /// ```rust
    /// let result = LogLevel::filter_level_string("danger");
    /// assert_eq!(result, "error");
    ///
    /// let result = LogLevel::filter_level_string("warning");
    /// assert_eq!(result, "warn");
    ///
    /// let result = LogLevel::filter_level_string("");
    /// assert_eq!(result, "info");
    ///
    /// let result = LogLevel::filter_level_string("trace message");
    /// assert_eq!(result, "trace");
    /// ```
    ///
    /// # Behavior
    ///
    /// - Direct matches:
    ///   - `"danger"` maps to `"error"`.
    /// - Partial matches:
    ///   - Strings containing `"err"` or `"dang"` map to `"error"`.
    ///   - Strings containing `"war"` map to `"warn"`.
    ///   - Strings containing `"inf"` map to `"info"`.
    ///   - Strings containing `"deb"` map to `"debug"`.
    ///   - Strings containing `"tra"` map to `"trace"`.
    ///   - Strings containing `"off"` map to `"off"`.
    /// - Empty or unrecognized strings default to `"off"`.
    pub fn filter_level_string<T: AsRef<str>>(level: T) -> &'static str
    {
        let level = level.as_ref().to_lowercase();
        match level.trim() {
            "danger" => "error",
            "info" => "info",
            "warning" => "warn",
            "warn" => "warn",
            "critical" => "error",
            "error" => "error",
            "debug" => "debug",
            "trace" => "trace",
            "off" => "off",
            "disable" => "off",
            _ => {
                if level.is_empty() {
                    return "info";
                }
                if level.contains("err") || level.contains("dang") || level.contains("crit") {
                    return "error";
                }
                if level.contains("war") {
                    return "warn";
                }
                if level.contains("inf") {
                    return "info";
                }
                if level.contains("deb") {
                    return "debug";
                }
                if level.contains("tra") {
                    return "trace";
                }
                if level.contains("of") {
                    return "off";
                }
                "off"
            }
        }
    }

    /// Converts a string representation of a logging level to the corresponding `LevelFilter`.
    ///
    /// # Parameters
    /// - `level`: A string slice representing the desired logging level. Accepted values are:
    ///   - `"error"`
    ///   - `"warn"`
    ///   - `"info"`
    ///   - `"debug"`
    ///   - `"trace"`
    ///
    /// # Returns
    /// - A `LevelFilter` variant corresponding to the provided log level.
    /// - Returns `LevelFilter::Off` if no valid level is provided or the input does not match any known levels.
    ///
    /// # Behavior
    /// The method normalizes the input string using `filter_level_string` before matching it against
    /// known logging levels. The levels correspond to the following:
    /// - `"error"` maps to `LevelFilter::Error`
    /// - `"warn"` maps to `LevelFilter::Warn`
    /// - `"info"` maps to `LevelFilter::Info`
    /// - `"debug"` maps to `LevelFilter::Debug`
    /// - `"trace"` maps to `LevelFilter::Trace`
    /// - Any other string maps to `LevelFilter::Off`.
    ///
    /// # Example
    /// ```rust
    /// use your_crate::YourStruct;
    /// use log::LevelFilter;
    ///
    /// let level_filter = YourStruct::level_filter("info");
    /// assert_eq!(level_filter, LevelFilter::Info);
    ///
    /// let invalid_filter = YourStruct::level_filter("invalid");
    /// assert_eq!(invalid_filter, LevelFilter::Off);
    /// ```
    pub fn level_filter<T: AsRef<str>>(level: T) -> LevelFilter {
        let level = Self::filter_level_string(level);
        match level {
            "error" => LevelFilter::Error,
            "warn" => LevelFilter::Warn,
            "info" => LevelFilter::Info,
            "debug" => LevelFilter::Debug,
            "trace" => LevelFilter::Trace,
            _ => LevelFilter::Off,
        }
    }

    /// Determines the appropriate logging level filter based on the provided string input.
    ///
    /// # Arguments
    ///
    /// * `level` - A `&str` representing the desired logging level. This can be a case-insensitive
    ///   string that describes standard log levels (e.g., "error", "info", "warn") or custom
    ///   strings containing similar key patterns.
    /// * `default` - A `LevelFilter` that serves as the default logging level filter when
    ///   the input does not match any known pattern.
    ///
    /// # Returns
    ///
    /// A `LevelFilter` corresponding to the input string, or the provided `default` if no
    /// known level is matched.
    ///
    /// # Supported Log Levels
    ///
    /// - `"trace"`  -> `LevelFilter::Trace`
    /// - `"debug"`  -> `LevelFilter::Debug`
    /// - `"info"`   -> `LevelFilter::Info`
    /// - `"warning"` or `"warn"` -> `LevelFilter::Warn`
    /// - `"error"`, `"danger"`, `"critical"` -> `LevelFilter::Error`
    /// - `"off"` or `"disable"` -> `LevelFilter::Off`
    ///
    /// # Fuzzy Matching
    ///
    /// If the provided string does not match exactly with any of the predefined levels,
    /// partial matches are attempted using common substrings:
    ///
    /// - Strings containing `"err"`, `"dang"`, or `"crit"` map to `LevelFilter::Error`.
    /// - Strings containing `"war"` map to `LevelFilter::Warn`.
    /// - Strings containing `"inf"` map to `LevelFilter::Info`.
    /// - Strings containing `"deb"` map to `LevelFilter::Debug`.
    /// - Strings containing `"tra"` map to `LevelFilter::Trace`.
    /// - Strings containing `"of"` or `"no"` map to `LevelFilter::Off`.
    ///
    /// # Edge Cases
    ///
    /// If the `level` is an empty string, the function defaults to `LevelFilter::Info`.
    /// If no match is found after substring checks, the function returns the `default`
    /// `LevelFilter` supplied as an argument.
    ///
    /// # Examples
    ///
    /// ```rust
    /// use log::LevelFilter;
    ///
    /// let filter = level_filter_default("INFO", LevelFilter::Error);
    /// assert_eq!(filter, LevelFilter::Info);
    ///
    /// let filter = level_filter_default("warNING", LevelFilter::Off);
    /// assert_eq!(filter, LevelFilter::Warn);
    ///
    /// let filter = level_filter_default("unknown", LevelFilter::Trace);
    /// assert_eq!(filter, LevelFilter::Trace);
    ///
    /// let filter = level_filter_default("", LevelFilter::Error);
    /// assert_eq!(filter, LevelFilter::Info);
    /// ```
    pub fn level_filter_default<T: AsRef<str>>(level: T, default: LevelFilter) -> LevelFilter
    {
        let level = level.as_ref().trim().to_lowercase();
        match level.as_str() {
            "danger" => LevelFilter::Error,
            "info" => LevelFilter::Info,
            "warning" => LevelFilter::Warn,
            "warn" => LevelFilter::Warn,
            "critical" => LevelFilter::Error,
            "error" => LevelFilter::Error,
            "debug" => LevelFilter::Debug,
            "trace" => LevelFilter::Trace,
            "off" => LevelFilter::Off,
            "disable" => LevelFilter::Off,
            _ => {
                if level.is_empty() {
                    return LevelFilter::Info;
                }
                if level.contains("err") || level.contains("dang") || level.contains("crit") {
                    return LevelFilter::Error;
                }
                if level.contains("war") {
                    return LevelFilter::Warn;
                }
                if level.contains("inf") {
                    return LevelFilter::Info;
                }
                if level.contains("deb") {
                    return LevelFilter::Debug;
                }
                if level.contains("tra") {
                    return LevelFilter::Trace;
                }
                if level.contains("of") || level.contains("no") {
                    return LevelFilter::Off;
                }
                default
            }
        }
    }

    /// Converts a log level of generic type `T` into a corresponding string representation.
    ///
    /// # Type Parameters
    /// - `T`: A type that implements the `Into<LevelFilter>` trait, which means it can be
    ///        converted into the `LevelFilter` enum.
    ///
    /// # Arguments
    /// - `level`: A value of type `T` that represents the log level to be converted into a string.
    ///
    /// # Returns
    /// - A static string slice (`&'static str`) that corresponds to the provided log level.
    ///
    /// # Log Level Mappings
    /// The function matches the log levels from the `LevelFilter` enum to their respective string representations:
    /// - `LevelFilter::Off` -> `"off"`
    /// - `LevelFilter::Error` -> `"error"`
    /// - `LevelFilter::Warn` -> `"warning"`
    /// - `LevelFilter::Info` -> `"info"`
    /// - `LevelFilter::Debug` -> `"debug"`
    /// - `LevelFilter::Trace` -> `"trace"`
    ///
    /// # Example
    /// ```rust
    /// use log::LevelFilter;
    ///
    /// let level = LevelFilter::Error;
    /// let level_str = level_string(level);
    /// assert_eq!(level_str, "error");
    ///
    /// let level = LevelFilter::Debug;
    /// let level_str = level_string(level);
    /// assert_eq!(level_str, "debug");
    /// ```
    ///
    /// # Notes
    /// - The function relies on the `Into<LevelFilter>` trait for the conversion. Ensure that the
    ///   provided type `T` implements this trait to avoid compilation errors.
    /// - The return strings are all lowercase and correspond to the conventional names of logging levels.
    ///
    /// # Errors
    /// - This function does not return any errors. Input values that cannot be converted into a
    ///   `LevelFilter` will result in a compile-time error.
    pub fn level_string<T>(level: T) -> &'static str
    where
        T: Into<LevelFilter>,
    {
        match &level.into() {
            LevelFilter::Off => "off",
            LevelFilter::Error => "error",
            LevelFilter::Warn => "warning",
            LevelFilter::Info => "info",
            LevelFilter::Debug => "debug",
            LevelFilter::Trace => "trace",
        }
    }
}
