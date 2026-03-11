use crate::cores::helper::year_month_day::YearMonthDay;
use crate::cores::system::error::{Error, ResultError};
use chrono::Datelike;
use parking_lot::{Mutex, RwLock};
use reqwest::Client;
use std::collections::{HashMap, HashSet};
use std::io::{Read, Write};
use std::net::TcpStream;
use std::sync::{Arc, LazyLock};
use std::time::Duration;
use tokio::sync::oneshot;

#[derive(Debug, Clone)]
pub struct TldRequestRecord {
    pub init: bool,
    pub timestamp: u64,
    pub succeed: bool,
    pub fail_count: u64,
    pub year_month_day: YearMonthDay,
    pub records: Arc<Vec<String>>,
}

pub const TLD_URL: &str = "https://data.iana.org/TLD/tlds-alpha-by-domain.txt";
pub const IANA_WHOIS_SERVER: &str = "whois.iana.org";
pub const IANA_WHOIS_PORT: u16 = 43;

fn parse_version_timestamp_tld(content: &str) -> YearMonthDay {
    let first_line = content.trim_start().lines().next().unwrap_or("");
    if !first_line.starts_with('#') {
        return YearMonthDay::zero();
    }
    let first_line = first_line.trim_start_matches('#').to_lowercase();
    let split = first_line.split_whitespace().collect::<Vec<&str>>();
    if split.len() < 2 || split[0] != "version" {
        return YearMonthDay::zero();
    }
    let version = split[1];
    if !version.starts_with("20") || version.len() < 8 || !version.parse::<u64>().is_ok() {
        return YearMonthDay::zero();
    }
    let year: i32 = version[0..4].parse().unwrap_or(0);
    let month: u32 = version[4..6].parse().unwrap_or(0);
    let day: u32 = version[6..8].parse().unwrap_or(0);
    YearMonthDay::untouched(year, month, day)
}

static TLD_CACHE: LazyLock<RwLock<TldRequestRecord>> = LazyLock::new(|| {
    let mut tld = HashSet::new();
    let content = include_str!("../../../resources/idna/tlds-alpha-by-domain.txt");
    let year_month_day = parse_version_timestamp_tld(content);
    for l in content.lines() {
        let l = l.trim();
        if l.is_empty() || l.starts_with('#') {
            continue;
        }
        tld.insert(l.to_uppercase().to_string());
    }
    RwLock::new(TldRequestRecord {
        init: true,
        timestamp: 0,
        succeed: true,
        year_month_day,
        fail_count: 0,
        records: Arc::new(tld.into_iter().collect::<Vec<String>>()),
    })
});

static WHOIS_SERVER_CACHE: LazyLock<RwLock<Option<HashMap<String, String>>>> =
    LazyLock::new(|| RwLock::new(None));

static QUEUE: Mutex<Vec<oneshot::Sender<ResultError<TldRequestRecord>>>> = Mutex::new(Vec::new());
static IS_FETCHING: Mutex<bool> = Mutex::new(false);

#[derive(Debug)]
pub struct Tld;

impl Tld {
    pub async fn fetch() -> ResultError<TldRequestRecord> {
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
            let result = Self::do_actual_fetch().await;
            let mut q = QUEUE.lock();
            let mut fetching = IS_FETCHING.lock();

            for sender in q.drain(..) {
                let res_to_send = match &result {
                    Ok(data) => Ok(data.clone()),
                    Err(e) => Err(Error::other(e.to_string())),
                };
                let _ = sender.send(res_to_send);
            }
            *fetching = false;
        }

        rx.await.map_err(|_| Error::channel_closed("TLD fetch channel closed"))?
    }

    pub fn is_need_update(interval: Option<u64>) -> bool {
        let cached = TLD_CACHE.read();
        if cached.init || cached.timestamp == 0 {
            return true;
        }
        let utc = chrono::Utc::now();
        if cached.year_month_day.year() > utc.year() {
            return true;
        }
        if let Some(interval_val) = interval {
            let interval_added = cached.timestamp + interval_val;
            interval_added < (utc.timestamp() as u64)
        } else {
            cached.year_month_day.less_than(&YearMonthDay::today())
        }
    }

    pub async fn update(force: bool) -> ResultError<TldRequestRecord> {
        if !force && !Self::is_need_update(None) {
            return Ok(TLD_CACHE.read().clone());
        }
        Self::fetch().await
    }

    async fn do_actual_fetch() -> ResultError<TldRequestRecord> {
        let response = Client::builder()
            .timeout(Duration::from_secs(10))
            .build()
            .map_err(Error::from_error)?
            .get(TLD_URL)
            .send()
            .await
            .map_err(Error::from_error)?
            .text()
            .await
            .map_err(Error::from_error)?;

        let tld = response
            .lines()
            .filter(|l| {
                let l = l.trim();
                !l.is_empty() && !l.starts_with('#')
            })
            .map(|l| l.to_uppercase().to_string())
            .collect::<Vec<String>>();

        let now = chrono::Utc::now();
        let record = TldRequestRecord {
            init: false,
            timestamp: now.timestamp() as u64,
            succeed: true,
            fail_count: 0,
            records: Arc::new(tld),
            year_month_day: YearMonthDay::from(now),
        };

        {
            let mut cache = TLD_CACHE.write();
            *cache = record.clone();
        }
        Ok(record)
    }

    fn parse_extension<T: AsRef<str>>(ext: T) -> String {
        ext.as_ref()
            .split('.')
            .last()
            .unwrap_or("")
            .to_uppercase()
    }

    pub fn whois_server<T: AsRef<str>>(tld: T) -> ResultError<String> {
        let tld_ext = Self::parse_extension(tld);
        if !Self::list().contains(&tld_ext) {
            return Err(Error::invalid_input(format!("Invalid TLD: {}", tld_ext)));
        }

        {
            let cache = WHOIS_SERVER_CACHE.read();
            if let Some(servers) = cache.as_ref() {
                if let Some(server) = servers.get(&tld_ext) {
                    return Ok(server.clone());
                }
            }
        }

        let mut stream = TcpStream::connect(format!("{}:{}", IANA_WHOIS_SERVER, IANA_WHOIS_PORT))
            .map_err(|_| Error::invalid_input("Failed to connect to IANA WHOIS Server"))?;

        stream.set_read_timeout(Some(Duration::from_secs(10))).map_err(Error::from_error)?;
        stream.set_write_timeout(Some(Duration::from_secs(10))).map_err(Error::from_error)?;
        stream.set_nodelay(true).map_err(Error::from_error)?;

        let query = format!("{}\r\n", tld_ext);
        stream.write_all(query.as_bytes()).map_err(|_| Error::invalid_input("Failed to write query"))?;

        let mut response = String::new();
        stream.read_to_string(&mut response).map_err(Error::from_error)?;
        let _ = stream.shutdown(std::net::Shutdown::Both);

        for line in response.lines() {
            if line.to_lowercase().starts_with("whois:") {
                let server = line
                    .split(':')
                    .nth(1)
                    .ok_or_else(|| Error::invalid_input("Invalid WHOIS response format"))?
                    .trim()
                    .to_lowercase();

                let mut cache = WHOIS_SERVER_CACHE.write();
                cache.get_or_insert_with(HashMap::new).insert(tld_ext.clone(), server.clone());
                return Ok(server);
            }
        }
        Err(Error::invalid_input("Failed to parse whois server from IANA response"))
    }

    pub fn year_month_day() -> YearMonthDay {
        TLD_CACHE.read().year_month_day.clone()
    }

    pub fn timestamp() -> u64 {
        TLD_CACHE.read().timestamp
    }

    pub fn list() -> Arc<Vec<String>> {
        TLD_CACHE.read().records.clone()
    }
}
