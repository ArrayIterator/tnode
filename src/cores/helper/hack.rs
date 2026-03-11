use crate::cores::system::error::{Error, ResultError};
use coarsetime::Clock;
use comfy_table::presets::UTF8_FULL;
use comfy_table::Width::Percentage;
use comfy_table::{Cell, Color, ColumnConstraint, ContentArrangement, Table};
use core::iter::Iterator;
use nix::libc;
use ratatui::style::Color as RataTuiColor;
use serde::{Deserialize, Serialize};
use std::collections::HashSet;
use std::time::Duration;

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
pub struct MemoryStat {
    pub vm_rss: usize,   // Resident Set Size (current)
    pub vm_size: usize,  // Virtual memory size
    pub vm_data: usize,  // Data segment size
    pub vm_hwm: usize,   // High water mark (peak RSS)
    pub rss_anon: usize, // Anonymous RSS
    pub rss_file: usize, // File-backed RSS
}

pub struct Hack;

impl Hack {
    #[inline(always)]
    pub fn time() -> u32 {
        Clock::now_since_epoch().as_secs() as u32
    }

    #[inline(always)]
    pub fn hrtime() -> u64 {
        let mut ts = libc::timespec {
            tv_sec: 0,
            tv_nsec: 0,
        };
        unsafe {
            if libc::clock_gettime(libc::CLOCK_MONOTONIC, &mut ts) != 0 {
                return 0;
            }
        }
        (ts.tv_sec as u64) * 1_000_000_000 + (ts.tv_nsec as u64)
    }

    fn clean_quoted_identifier<T: Into<String>>(input: T) -> String {
        let input = input.into();
        let mut result = String::new();
        let mut chars = input.chars().peekable();
        let mut in_quotes = false;

        while let Some(c) = chars.next() {
            match c {
                '"' => {
                    if in_quotes {
                        // Peek to see if the next char is also a double quote (escaped quote)
                        if chars.peek() == Some(&'"') {
                            result.push('"'); // It's an escaped quote: "" -> "
                            chars.next();     // Consume the second quote
                        } else {
                            in_quotes = false; // End of a quoted segment
                        }
                    } else {
                        in_quotes = true; // Start of a quoted segment
                    }
                }
                '.' if !in_quotes => {
                    // Real separator between identifiers
                    result.push('.');
                }
                _ => {
                    // Regular character
                    result.push(c);
                }
            }
        }
        result
    }

    pub fn escape_table_identifier<T: Into<String>>(input: T, quoted: bool) -> String {
        let input = Self::clean_quoted_identifier(input);
        let mut escaped = String::new();
        if quoted {
            escaped.push('"');
        }
        for c in input.chars() {
            if c == '"' {
                escaped.push('"');
                escaped.push('"');
            } else {
                escaped.push(c);
            }
        }
        if quoted {
            escaped.push('"');
        }
        escaped
    }

    pub fn escape_table_identifier_quote<T: Into<String>>(input: T) -> String {
        Self::escape_table_identifier(input, true)
    }

    pub fn escape_table_identifier_unquote<T: Into<String>>(input: T) -> String {
        Self::escape_table_identifier(input, false)
    }

    pub fn unique_set_string(inherits: Vec<String>) -> Vec<String> {
        if inherits.is_empty() {
            return inherits;
        }
        let mut seen = HashSet::with_capacity(inherits.len());
        let mut result = Vec::with_capacity(inherits.len());
        for value in inherits {
            if seen.insert(value.clone()) {
                result.push(value);
            }
        }
        result
    }

    pub fn comfy_color_to_rata_tui(color: Color) -> ratatui::style::Color {
        match color {
            Color::Reset => RataTuiColor::Reset,
            Color::Black => RataTuiColor::Black,
            Color::DarkGrey => RataTuiColor::DarkGray,
            Color::Red => RataTuiColor::LightRed,
            Color::DarkRed => RataTuiColor::Red,
            Color::Green => RataTuiColor::LightGreen,
            Color::DarkGreen => RataTuiColor::Green,
            Color::Yellow => RataTuiColor::LightYellow,
            Color::DarkYellow => RataTuiColor::Yellow,
            Color::Blue => RataTuiColor::LightBlue,
            Color::DarkBlue => RataTuiColor::Blue,
            Color::Magenta => RataTuiColor::LightMagenta,
            Color::DarkMagenta => RataTuiColor::Magenta,
            Color::Cyan => RataTuiColor::LightCyan,
            Color::DarkCyan => RataTuiColor::Cyan,
            Color::White => RataTuiColor::White,
            Color::Grey => RataTuiColor::Gray,
            Color::Rgb { r, g, b } => RataTuiColor::Rgb(r, g, b),
            Color::AnsiValue(e) => RataTuiColor::Indexed(e),
        }
    }

    pub fn rata_tui_color_to_comfy(color: ratatui::style::Color) -> Color {
        match color {
            ratatui::style::Color::Reset => Color::Reset,
            ratatui::style::Color::Black => Color::Black,
            ratatui::style::Color::Red => Color::DarkRed,
            ratatui::style::Color::Green => Color::DarkGreen,
            ratatui::style::Color::Yellow => Color::DarkYellow,
            ratatui::style::Color::Blue => Color::DarkBlue,
            ratatui::style::Color::Magenta => Color::DarkMagenta,
            ratatui::style::Color::Cyan => Color::DarkCyan,
            ratatui::style::Color::Gray => Color::Grey,
            ratatui::style::Color::DarkGray => Color::DarkGrey,
            ratatui::style::Color::LightRed => Color::Red,
            ratatui::style::Color::LightGreen => Color::Green,
            ratatui::style::Color::LightYellow => Color::Yellow,
            ratatui::style::Color::LightBlue => Color::Blue,
            ratatui::style::Color::LightMagenta => Color::Magenta,
            ratatui::style::Color::LightCyan => Color::Cyan,
            ratatui::style::Color::White => Color::White,
            ratatui::style::Color::Rgb(r, g, b) => Color::Rgb { r, g, b },
            ratatui::style::Color::Indexed(e) => Color::AnsiValue(e),
        }
    }
    pub fn release_memory_to_os() {
        #[cfg(target_os = "linux")]
        unsafe {
            libc::malloc_trim(0);
        }
    }

    pub fn size_to_bytes<T: AsRef<str>>(size: T) -> ResultError<isize> {
        let s = size.as_ref().trim();

        if s.is_empty() {
            return Err(Error::parse_error("empty size"));
        }

        let mut number_part = String::new();
        let mut unit_part = String::new();

        for c in s.chars() {
            if c.is_ascii_digit() {
                if unit_part.is_empty() {
                    number_part.push(c);
                } else {
                    return Err(Error::parse_error("invalid size format"));
                }
            } else if c.is_ascii_whitespace() {
                continue;
            } else {
                unit_part.push(c);
            }
        }

        if number_part.is_empty() {
            return Err(Error::parse_error("missing number"));
        }

        let value: isize = number_part
            .parse()
            .map_err(|_| Error::parse_error("invalid number"))?;

        let multiplier: isize = match unit_part.to_ascii_lowercase().as_str() {
            "" | "b" => 1,
            "k" | "kb" => 1024,
            "m" | "mb" => 1024_i64.pow(2) as isize,
            "g" | "gb" => 1024_i64.pow(3) as isize,
            "t" | "tb" => 1024_i64.pow(4) as isize,
            "p" | "pb" => 1024_i64.pow(5) as isize,
            "e" | "eb" => 1024_i64.pow(6) as isize,
            "z" | "zb" => 1024_i64.pow(7) as isize,
            "y" | "yb" => 1024_i64.pow(8) as isize,
            _ => return Err(Error::parse_error("unknown unit")),
        };

        Ok(value.saturating_mul(multiplier))
    }

    pub fn size_to_bytes_zero<T: AsRef<str>>(size: T) -> usize {
        let s = size.as_ref().trim();

        if s.is_empty() {
            return 0;
        }

        let mut number_part = String::new();
        let mut unit_part = String::new();
        for c in s.chars() {
            if c.is_ascii_digit() {
                if unit_part.is_empty() {
                    number_part.push(c);
                } else {
                    break;
                }
            } else if c.is_ascii_whitespace() {
                continue;
            } else {
                unit_part.push(c);
            }
        }
        if number_part.is_empty() {
            return 0; // fallback
        }

        let value: isize = number_part.parse::<isize>().unwrap_or(0);
        if value <= 0 {
            return 0;
        }
        let value = value.abs() as usize;
        let unit_part = unit_part.to_ascii_lowercase();
        value.saturating_mul(Self::multiply_byte(unit_part))
    }

    fn multiply_byte<T: AsRef<str>>(unit_part: T) -> usize {
        let unit_part = unit_part.as_ref().trim();
        match unit_part {
            "" | "b" => 1,
            "k" | "kb" => 1024,
            "m" | "mb" => 1024_i64.pow(2) as usize,
            "g" | "gb" => 1024_i64.pow(3) as usize,
            "t" | "tb" => 1024_i64.pow(4) as usize,
            "p" | "pb" => 1024_i64.pow(5) as usize,
            "e" | "eb" => 1024_i64.pow(6) as usize,
            "z" | "zb" => 1024_i64.pow(7) as usize,
            "y" | "yb" => 1024_i64.pow(8) as usize,
            _ => {
                if unit_part.len() == 1 {
                    return 1;
                }
                if let Some(on_char) = unit_part.chars().next() {
                    let str = on_char.to_string();
                    if str.is_empty() {
                        return 1;
                    }
                    return Self::multiply_byte(str);
                }
                1
            }
        }
    }

    pub fn format_size(size: usize) -> String {
        Self::format_size_with_precision(size, 2, true)
    }

    pub fn format_size_trim(size: usize) -> String {
        Self::format_size_with_precision(size, 2, false)
    }

    pub fn format_size_with_precision(size: usize, precision: usize, keep_precision: bool) -> String {
        let mut size = size as f64;
        let units = ["B", "KB", "MB", "GB", "TB"];
        let mut formatted = format!("{:.1$}", size, precision);
        let mut unit = "PB";
        for (_, u) in units.iter().enumerate() {
            if size < 1024.0 {
                unit = u;
                formatted = format!("{:.1$}", size, precision);
                break;
            }
            size /= 1024.0;
        }
        if !keep_precision {
            formatted = formatted.trim_end_matches('0').trim_end_matches('.').to_string();
        }
        format!("{} {}", formatted, unit)
    }

    pub fn parse_multi_duration<T: AsRef<str>>(s: T) -> Vec<(String, String)> {
        let mut results = Vec::new();
        let mut current_num = String::new();
        let mut current_unit = String::new();
        let mut last_was_digit = true;
        let s = s.as_ref().trim().replace(',', " ");

        for c in s.chars() {
            if c.is_whitespace() {
                continue;
            }
            if c.is_ascii_digit() {
                if !last_was_digit && !current_num.is_empty() {
                    let unit = if current_unit.is_empty() {
                        "s".to_string()
                    } else {
                        current_unit.clone()
                    };
                    results.push((current_num.clone(), unit));

                    current_num.clear();
                    current_unit.clear();
                }
                current_num.push(c);
                last_was_digit = true;
            } else {
                current_unit.push(c);
                last_was_digit = false;
            }
        }

        if !current_num.is_empty() {
            let unit = if current_unit.is_empty() {
                "s".to_string()
            } else {
                current_unit
            };
            results.push((current_num, unit));
        }
        results
    }

    pub fn string_to_duration_compat<T: AsRef<str>>(duration: T) -> Duration {
        let s = duration.as_ref().trim();
        if s.is_empty() {
            return Duration::ZERO;
        }
        if let Ok(dur) = humantime::parse_duration(s) {
            return dur;
        }
        let mut dur = Duration::ZERO;
        for (size, unit) in Self::parse_multi_duration(s) {
            let current = Self::decouple_duration_second_compat(size.parse().unwrap_or(0), unit);
            dur += current;
        }
        dur
    }

    fn decouple_duration_second_compat<T: AsRef<str>>(size: u64, unit_part: T) -> Duration {
        let unit = unit_part.as_ref().trim();
        if unit.is_empty() {
            return Duration::from_secs(size);
        }
        // CEK CASE-SENSITIVE
        match unit {
            "M" => return Duration::from_secs(size * 2592000), // 30 days
            "m" => return Duration::from_secs(size * 60),      // 1 minute
            _ => {}
        } // CASE-INSENSITIVE (LOWERCASE)
        let u = unit.to_lowercase();
        match u.as_str() {
            // Nanoseconds
            "n" | "ns" | "nanos" => Duration::from_nanos(size),
            // Microseconds
            u if u.starts_with("mic")
                || u.starts_with("mc")
                || u.starts_with("us")
                || u.starts_with('μ') =>
            {
                Duration::from_micros(size)
            }
            // Milliseconds
            u if u == "ml" || u.starts_with("mil") || u.starts_with("ms") => {
                Duration::from_millis(size)
            }
            // Seconds
            u if u == "s" || u.starts_with("se") => Duration::from_secs(size),
            // Minutes
            u if u == "mn" || u.starts_with("min") => Duration::from_secs(size * 60),
            // Hours
            u if u.starts_with('h') => Duration::from_secs(size * 3600),
            // Days
            u if u.starts_with('d') => Duration::from_secs(size * 86400),
            // Weeks
            u if u.starts_with('w') => Duration::from_secs(size * 604800),
            // Months (Fallback case-insensitive)
            u if u.starts_with("mnt") || u.starts_with("mo") || u.starts_with("mt") => {
                Duration::from_secs(size * 2592000)
            }
            // Years
            u if u.starts_with('y') => Duration::from_secs(size * 31536000),
            // fallback compat
            u if u.starts_with("mi") => Duration::from_millis(size),
            u if u.starts_with('n') => Duration::from_nanos(size),
            u if u.starts_with('m') => Duration::from_mins(size),
            _ => Duration::from_secs(size),
        }
    }

    pub fn format_duration(duration: Duration, short: bool) -> String {
        let ns = duration.as_nanos() as f64;

        const NS_PER_US: f64 = 1_000.0;
        const NS_PER_MS: f64 = 1_000_000.0;
        const NS_PER_S: f64 = 1_000_000_000.0;
        const NS_PER_M: f64 = 60.0 * NS_PER_S;
        const NS_PER_H: f64 = 60.0 * NS_PER_M;

        let (value, unit) = if ns < NS_PER_US {
            (
                ns,
                if short {
                    "ns"
                } else if ns == 1.0 {
                    " nanosecond"
                } else {
                    " nanoseconds"
                },
            )
        } else if ns < NS_PER_MS {
            let v = ns / NS_PER_US;
            (
                v,
                if short {
                    "us"
                } else if v == 1.0 {
                    " microsecond"
                } else {
                    " microseconds"
                },
            )
        } else if ns < NS_PER_S {
            let v = ns / NS_PER_MS;
            (
                v,
                if short {
                    "ms"
                } else if v == 1.0 {
                    " millisecond"
                } else {
                    " milliseconds"
                },
            )
        } else if ns < NS_PER_M {
            let v = ns / NS_PER_S;
            (
                v,
                if short {
                    "s"
                } else if v == 1.0 {
                    " second"
                } else {
                    " seconds"
                },
            )
        } else if ns < NS_PER_H {
            let v = ns / NS_PER_M;
            (
                v,
                if short {
                    "m"
                } else if v == 1.0 {
                    " minute"
                } else {
                    " minutes"
                },
            )
        } else {
            let v = ns / NS_PER_H;
            (
                v,
                if short {
                    "h"
                } else if v == 1.0 {
                    " hour"
                } else {
                    " hours"
                },
            )
        };

        let formatted = format!("{:.2}", value);
        let trimmed = formatted.trim_end_matches('0').trim_end_matches('.');

        if short {
            format!("{} {}", trimmed, unit)
        } else {
            format!("{}{}", trimmed, unit)
        }
    }

    pub fn format_human_time_second(duration: usize, short: bool) -> String {
        if duration == 0 {
            return if short {
                "0s".to_string()
            } else {
                "0 seconds".to_string()
            };
        }

        let days = duration / 86400;
        let hours = (duration % 86400) / 3600;
        let minutes = (duration % 3600) / 60;
        let seconds = duration % 60;

        let mut parts = Vec::new();

        let units = [
            (days, "day", "d"),
            (hours, "hour", "h"),
            (minutes, "minute", "m"),
            (seconds, "second", "s"),
        ];

        for (value, long_unit, short_unit) in units {
            if value > 0 {
                if short {
                    parts.push(format!("{}{}", value, short_unit));
                } else {
                    parts.push(format!(
                        "{} {}{}",
                        value,
                        long_unit,
                        if value > 1 { "s" } else { "" }
                    ));
                }
            }
        }

        if short {
            parts.join(" ")
        } else {
            parts.join(", ")
        }
    }

    pub fn create_table(full_size: bool) -> Table {
        let mut table = Table::new();
        table
            .load_preset(UTF8_FULL)
            .set_content_arrangement(ContentArrangement::DynamicFullWidth)
            .set_header(vec![
                Cell::new("Category")
                    .fg(Color::Cyan)
                    .add_attribute(comfy_table::Attribute::Bold),
                Cell::new("Key")
                    .fg(Color::Cyan)
                    .add_attribute(comfy_table::Attribute::Bold),
                Cell::new("Value")
                    .fg(Color::Cyan)
                    .add_attribute(comfy_table::Attribute::Bold),
            ]);
        if full_size {
            table.set_constraints(vec![
                ColumnConstraint::ContentWidth,
                ColumnConstraint::ContentWidth,
                ColumnConstraint::LowerBoundary(Percentage(50)),
            ]);
            match terminal_size::terminal_size() {
                None => {}
                Some((width, _)) => {
                    table.set_width(width.0);
                }
            }
        }

        table
    }

    pub fn str_to_hex<T: AsRef<[u8]>>(input: T) -> String {
        let bytes = input.as_ref();
        let mut out = String::with_capacity(bytes.len() * 2);
        for &b in bytes {
            use std::fmt::Write;
            write!(&mut out, "{:02x}", b).unwrap();
        }
        out
    }

    pub fn hex_to_bytes<T: AsRef<str>>(s: T) -> ResultError<Vec<u8>> {
        let s = s.as_ref();
        if s.len() % 2 != 0 {
            return Err(Error::invalid_input(format!(
                "Hex string length must be even: {}",
                s
            )));
        }
        Ok((0..s.len())
            .step_by(2)
            .map(|i| u8::from_str_radix(&s[i..i + 2], 16))
            .collect::<Result<Vec<_>, _>>()
            .map_err(|e| Error::parse_error(format!("Invalid hex string: {}", e)))?)
    }

    pub fn dec_to_hex<T: AsRef<str>>(str: T) -> String {
        format!("{:x}", str.as_ref().parse::<u64>().unwrap())
    }

    pub fn hex_to_dec<T: AsRef<str>>(str: T) -> String {
        u64::from_str_radix(str.as_ref(), 16).unwrap().to_string()
    }

    pub fn clean_unique_path<P: AsRef<str>>(path: P) -> String {
        let input = path.as_ref();
        let is_absolute = input.starts_with('/') || input.starts_with('\\');
        let normalized = input.replace('\\', "/");
        let mut parts = normalized
            .split('/')
            .filter(|p| !p.is_empty() && *p != ".")
            .collect::<Vec<&str>>();
        let mut vec = Vec::new();
        for p in parts {
            if p == ".." {
                vec.pop(); // Mundur satu folder
            } else {
                vec.push(p);
            }
        }
        let joined = vec.join("/");

        if is_absolute {
            format!("/{}", joined)
        } else if joined.is_empty() {
            ".".to_string()
        } else {
            joined
        }
    }
    pub fn clean_unique_path_trim<P: AsRef<str>>(path: P) -> String {
        Self::clean_unique_path(path)
            .trim_matches(|c| c == '/')
            .to_string()
    }
    pub fn is_numeric_string<P: AsRef<str>>(str: P) -> bool {
        str.as_ref().parse::<u64>().is_ok()
    }
}

pub trait IsNumeric {
    fn is_numeric(&self) -> bool;
}

impl<T: AsRef<[u8]>> IsNumeric for T {
    fn is_numeric(&self) -> bool {
        self.as_ref().iter().all(|b| b.is_ascii_digit())
    }
}

pub trait IsHex {
    fn is_hex(&self) -> bool;
}

impl<T: AsRef<[u8]>> IsHex for T {
    fn is_hex(&self) -> bool {
        self.as_ref().iter().all(|b| b.is_ascii_hexdigit())
    }
}

pub trait ToHex {
    fn to_hex(&self) -> String;
}

impl<T: AsRef<[u8]>> ToHex for T {
    fn to_hex(&self) -> String {
        self.as_ref()
            .iter()
            .map(|b| format!("{:02x}", b))
            .collect::<Vec<_>>()
            .join("")
    }
}
