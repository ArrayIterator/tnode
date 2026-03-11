use crate::cores::helper::year_month_day::YearMonthDay;
use crate::cores::system::error::{Error, ResultError};
use addr::domain::Name;
use addr::parser::DomainName;
use addr::publicsuffix::List;
use chrono::Datelike;
use parking_lot::{Mutex, RwLock};
use reqwest::Client;
use std::sync::{Arc, LazyLock};
use std::time::Duration;
use tokio::sync::oneshot;

pub const PSL_URL: &str = "https://publicsuffix.org/list/public_suffix_list.dat";

#[derive(Debug, Clone)]
pub struct PslRequestRecord {
    pub init: bool,
    pub timestamp: u64,
    pub succeed: bool,
    pub fail_count: u64,
    pub records: Arc<List>,
    pub year_month_day: YearMonthDay,
}

fn parse_version_timestamp_tld(content: &str) -> YearMonthDay {
    let mut found_version = "";
    for line in content.lines() {
        let line = line.trim();
        if line.is_empty() { continue; }
        if !line.starts_with("//") { break; }

        let l_line = line.trim_start_matches("//").trim_start();
        if l_line.to_lowercase().starts_with("version:") {
            found_version = l_line.trim_start_matches(|c: char| c.to_lowercase().next().unwrap() == 'v')
                                  .trim_start_matches(|c: char| c.to_lowercase().next().unwrap() == 'e')
                                  .trim_start_matches(|c: char| c.to_lowercase().next().unwrap() == 'r')
                                  .trim_start_matches(|c: char| c.to_lowercase().next().unwrap() == 's')
                                  .trim_start_matches(|c: char| c.to_lowercase().next().unwrap() == 'i')
                                  .trim_start_matches(|c: char| c.to_lowercase().next().unwrap() == 'o')
                                  .trim_start_matches(|c: char| c.to_lowercase().next().unwrap() == 'n')
                                  .trim_start_matches(':').trim();
            break;
        }
    }
    if found_version.is_empty() { return YearMonthDay::zero(); }

    let split = found_version.split('-').collect::<Vec<&str>>();
    if split.len() < 3 { return YearMonthDay::zero(); }

    let year_str = split[0];
    let month_str = split[1];

    let year: i32 = year_str.parse().unwrap_or(0);
    let month: u32 = month_str.parse().unwrap_or(0);

    if year == 0 || month == 0 || month > 12 { return YearMonthDay::zero(); }

    let mut data_str = String::new();
    for i in split[2].chars() {
        if i.is_numeric() { data_str.push(i); continue; }
        break;
    }

    if data_str.len() < 2 { return YearMonthDay::zero(); }
    let date = data_str[..2].parse::<u32>().unwrap_or(0);

    YearMonthDay::new(year, month, date)
}

static PSL_CACHE: LazyLock<RwLock<Option<PslRequestRecord>>> = LazyLock::new(|| {
    let content = include_str!("../../../resources/idna/public_suffix_list.dat");
    let year_month_day = parse_version_timestamp_tld(content);

    RwLock::new(content.parse::<List>().ok().map(|l| {
        let now = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .map(|d| d.as_secs())
            .unwrap_or(0);
        PslRequestRecord {
            year_month_day,
            init: true,
            timestamp: now,
            succeed: true,
            fail_count: 0,
            records: Arc::new(l),
        }
    }))
});

static QUEUE: Mutex<Vec<oneshot::Sender<ResultError<PslRequestRecord>>>> = Mutex::new(Vec::new());
static IS_FETCHING: Mutex<bool> = Mutex::new(false);

#[derive(Debug)]
pub struct PSL;

impl PSL {
    pub async fn fetch() -> ResultError<PslRequestRecord> {
        let (tx, rx) = oneshot::channel();
        let should_start_fetch = {
            let mut q = QUEUE.lock();
            let mut fetching = IS_FETCHING.lock();
            q.push(tx);
            if !*fetching {
                *fetching = true;
                true
            } else {
                false
            }
        };

        if should_start_fetch {
            let result = Self::do_actual_fetch(15).await;
            let mut q = QUEUE.lock();
            let mut fetching = IS_FETCHING.lock();
            for sender in q.drain(..) {
                let _ = sender.send(result.clone());
            }
            *fetching = false;
        }

        rx.await.map_err(|_| Error::channel_closed("PSL fetch channel closed"))?
    }

    pub fn is_need_update(interval: Option<u64>) -> bool {
        let cache_lock = PSL_CACHE.read();
        let Some(cached) = cache_lock.as_ref() else { return true };
        if cached.init || cached.timestamp == 0 { return true; }

        let utc = chrono::Utc::now();
        if cached.year_month_day.year() > utc.year() { return true; }

        if let Some(interval_val) = interval {
            (cached.timestamp + interval_val) < (utc.timestamp() as u64)
        } else {
            cached.year_month_day.less_than(&YearMonthDay::today())
        }
    }

    pub async fn update(force: bool) -> ResultError<PslRequestRecord> {
        if !force && !Self::is_need_update(None) {
            if let Some(cached) = PSL_CACHE.read().as_ref() {
                return Ok(cached.clone());
            }
        }
        Self::fetch().await
    }

    pub fn list() -> Arc<List> {
        PSL_CACHE.read().as_ref().unwrap().records.clone()
    }

    async fn do_actual_fetch(timeout: u64) -> ResultError<PslRequestRecord> {
        let client = Client::builder()
            .timeout(Duration::from_secs(timeout))
            .build()
            .map_err(Error::from_error)?;

        let response = client.get(PSL_URL).send().await.map_err(Error::from_error)?;
        let content = response.text().await.map_err(Error::from_error)?;

        let psl_list = content.parse::<List>()
            .map_err(|e| Error::parse_error(format!("Fail to parse PSL: {}", e)))?;

        let utc = chrono::Utc::now();
        let psl = PslRequestRecord {
            init: false,
            timestamp: utc.timestamp() as u64,
            succeed: true,
            fail_count: 0,
            records: Arc::new(psl_list),
            year_month_day: YearMonthDay::from(utc),
        };

        {
            let mut cache = PSL_CACHE.write();
            *cache = Some(psl.clone());
        }
        Ok(psl)
    }

    pub fn parse_domain_name(name: &str) -> ResultError<Name<'_>> {
        Self::list()
            .parse_domain_name(name)
            .map_err(|e| Error::parse_error(format!("Fail to parse domain name: {}", e)))
    }
}
