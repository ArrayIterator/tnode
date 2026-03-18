use std::time::Duration;

use chrono::Datelike;

pub enum Day {
    Monday,
    Tuesday,
    Wednesday,
    Thursday,
    Friday,
    Saturday,
    Sunday,
}

impl Day {
    pub fn from_i32(day: i32) -> Option<Self> {
        let day = day % 7 + 1;
        match day {
            1 => Some(Day::Monday),
            2 => Some(Day::Tuesday),
            3 => Some(Day::Wednesday),
            4 => Some(Day::Thursday),
            5 => Some(Day::Friday),
            6 => Some(Day::Saturday),
            7 => Some(Day::Sunday),
            _ => None,
        }
    }
    pub fn to_number(&self) -> u32 {
        match self {
            Day::Monday => 1,
            Day::Tuesday => 2,
            Day::Wednesday => 3,
            Day::Thursday => 4,
            Day::Friday => 5,
            Day::Saturday => 6,
            Day::Sunday => 7,
        }
    }
}

#[derive(Debug, Clone)]
pub struct YearMonthDay {
    year: i32,
    month: u32,
    day: u32,
}

impl Default for YearMonthDay {
    fn default() -> Self {
        let today = chrono::Utc::now();
        let year = today.year();
        let month = today.month();
        let day = today.day();
        Self { year, month, day }
    }
}

impl YearMonthDay {
    pub fn new(year: i32, month: u32, day: u32) -> Self {
        let mut year = year;
        let mut month = month;
        let mut day = day;
        while month > 12 {
            year += 1;
            month -= 12;
        }
        while month == 0 {
            year -= 1;
            month = 12;
        }

        while day == 0 {
            month -= 1;
            if month == 0 {
                year -= 1;
                month = 12;
            }
            day = Self::total_day_in_month(year, month);
        }
        loop {
            let max_day = Self::total_day_in_month(year, month);
            if day <= max_day || max_day == 0 {
                break;
            }
            day -= max_day;
            month += 1;
            if month > 12 {
                year += 1;
                month = 1;
            }
        }

        Self { year, month, day }
    }
    pub fn untouched(year: i32, month: u32, day: u32) -> Self {
        Self { year, month, day }
    }
    pub fn to_normalized(&self) -> Self {
        Self::new(self.year, self.month, self.day)
    }
    pub fn today() -> Self {
        Self::default()
    }
    pub fn zero() -> Self {
        Self {
            year: 0,
            month: 0,
            day: 0,
        }
    }
    pub fn year(&self) -> i32 {
        self.year
    }
    pub fn month(&self) -> u32 {
        self.month
    }
    pub fn day(&self) -> u32 {
        self.day
    }

    fn is_leap(year: i32) -> bool {
        (year % 4 == 0 && year % 100 != 0) || (year % 400 == 0)
    }
    fn total_day_in_month(year: i32, month: u32) -> u32 {
        match month {
            1 | 3 | 5 | 7 | 8 | 10 | 12 => 31,
            4 | 6 | 9 | 11 => 30,
            2 => {
                if Self::is_leap(year) {
                    29
                } else {
                    28
                }
            }
            _ => 0,
        }
    }
    pub fn is_leap_year(&self) -> bool {
        Self::is_leap(self.year)
    }
    pub fn valid(&self) -> bool {
        if self.year == 0 || self.month == 0 || self.day == 0 {
            return false;
        }
        if self.month > 12 {
            return false;
        }
        if self.days_in_month() < self.day {
            return false;
        }
        true
    }
    pub fn days_in_month(&self) -> u32 {
        Self::total_day_in_month(self.year, self.month)
    }

    pub fn days_in_year(&self) -> u32 {
        if self.is_leap_year() { 366 } else { 365 }
    }
    pub fn day_name(&self) -> Option<Day> {
        let y = self.year - 1;
        let mut total: i32 = y * 365
            + (y / 4)   // leap
            - (y / 100) // subtract leap for century
            + (y / 400); // add every 400 years
        let days_before_month = [0, 31, 59, 90, 120, 151, 181, 212, 243, 273, 304, 334];
        if self.month >= 1 && self.month <= 12 {
            total += days_before_month[(self.month - 1) as usize];
            if self.month > 2 && self.is_leap_year() {
                total += 1;
            }
        }
        total += (self.day - 1) as i32;
        // checking year
        Day::from_i32(total)
    }
    pub fn next_day(&self) -> Self {
        Self::new(self.year, self.month, self.day + 1)
    }
    pub fn prev_day(&self) -> Self {
        Self::new(self.year, self.month, self.day - 1)
    }
    pub fn next_week(&self) -> Self {
        Self::new(self.year, self.month, self.day + 7)
    }
    pub fn prev_week(&self) -> Self {
        if self.day > 7 {
            Self::new(self.year, self.month, self.day - 7)
        } else {
            let mut target_month = self.month;
            let mut target_year = self.year;
            if target_month == 1 {
                target_month = 12;
                target_year -= 1;
            } else {
                target_month -= 1;
            }
            let days_in_prev_month = Self::total_day_in_month(target_year, target_month);
            let target_day = (days_in_prev_month + self.day) - 7;

            Self::new(target_year, target_month, target_day)
        }
    }
    // returning same date on next month
    pub fn next_month(&self) -> Self {
        let mut target_month = self.month + 1;
        let mut target_year = self.year;

        if target_month > 12 {
            target_month = 1;
            target_year += 1;
        }

        let max_day_in_target = Self::total_day_in_month(target_year, target_month);

        let target_day = if self.day > max_day_in_target {
            max_day_in_target
        } else {
            self.day
        };

        Self::new(target_year, target_month, target_day)
    }

    // returning same date on prev month
    pub fn prev_month(&self) -> Self {
        let mut target_month = self.month;
        let mut target_year = self.year;

        if target_month == 1 {
            target_month = 12;
            target_year -= 1;
        } else {
            target_month -= 1;
        }
        let max_day_in_target = Self::total_day_in_month(target_year, target_month);
        let target_day = if self.day > max_day_in_target {
            max_day_in_target
        } else {
            self.day
        };

        Self::new(target_year, target_month, target_day)
    }
    pub fn next_year(&self) -> Self {
        Self::new(self.year + 1, self.month, self.day)
    }
    pub fn prev_year(&self) -> Self {
        Self::new(self.year - 1, self.month, self.day)
    }
    pub fn equal(&self, other: &Self) -> bool {
        self.year == other.year && self.month == other.month && self.day == other.day
    }
    pub fn less_than(&self, other: &Self) -> bool {
        self.year < other.year
            || (self.year == other.year && self.month < other.month)
            || (self.year == other.year && self.month == other.month && self.day < other.day)
    }
    pub fn greater_than(&self, other: &Self) -> bool {
        self.year > other.year || (self.year == other.year && self.month > other.month)
    }
    pub fn to_path_sufix(&self) -> String {
        format!("{:04}/{:02}/{:02}", self.year, self.month, self.day)
    }
}

impl From<&YearMonthDay> for YearMonthDay {
    fn from(year_month_day: &YearMonthDay) -> Self {
        Self {
            ..year_month_day.clone()
        }
    }
}

impl<T: Datelike + 'static> From<T> for YearMonthDay {
    fn from(date: T) -> Self {
        Self {
            year: date.year(),
            month: date.month(),
            day: date.day(),
        }
    }
}

pub const MINUTE_IN_SECONDS: u64 = 60;
pub const MINUTE_IN_HOURS: u64 = 60;
pub const DAY_IN_HOURS: u64 = 24;
pub const WEEK_IN_DAYS: u64 = 7;
pub const MONTH_IN_DAYS: u64 = 30;
pub const YEAR_IN_DAYS: u64 = 365;

pub const WEEK_IN_HOURS: u64 = DAY_IN_HOURS * WEEK_IN_DAYS;
pub const MONTH_IN_HOURS: u64 = DAY_IN_HOURS * MONTH_IN_DAYS;
pub const YEAR_IN_HOURS: u64 = DAY_IN_HOURS * YEAR_IN_DAYS;

pub const HOUR_IN_SECONDS: u64 = MINUTE_IN_HOURS * MINUTE_IN_SECONDS;
pub const DAY_IN_SECONDS: u64 = DAY_IN_HOURS * HOUR_IN_SECONDS;
pub const WEEK_IN_SECONDS: u64 = WEEK_IN_HOURS * HOUR_IN_SECONDS;
pub const MONTH_IN_SECONDS: u64 = MONTH_IN_HOURS * HOUR_IN_SECONDS;
pub const YEAR_IN_SECONDS: u64 = YEAR_IN_HOURS * HOUR_IN_SECONDS;

// Gunakan from_secs agar bisa masuk ke const
pub const DURATION_HOUR: Duration = Duration::from_secs(HOUR_IN_SECONDS);
pub const DURATION_DAY: Duration = Duration::from_secs(DAY_IN_SECONDS);
pub const DURATION_WEEK: Duration = Duration::from_secs(WEEK_IN_SECONDS);
pub const DURATION_MONTH: Duration = Duration::from_secs(MONTH_IN_SECONDS);
pub const DURATION_YEAR: Duration = Duration::from_secs(YEAR_IN_SECONDS);
