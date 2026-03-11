use crate::cores::generator::uuid::Uuid;
use crate::cores::system::error::{Error, ResultError};
use crate::cores::system::runtime::Runtime;
use actix_multipart::Multipart as ActixMultipart;
use chrono::Datelike;
use dashmap::DashMap;
use futures_util::stream::StreamExt;
use futures_util::TryStreamExt;
use std::collections::HashMap;
use std::io::Write;
use std::path::PathBuf;
use std::sync::LazyLock;
use std::time::Instant;

#[derive(Debug)]
pub struct SavedFile {
    pub name: String,
    pub path: PathBuf,
    pub size: u64,
    pub content_type: Option<String>,
    pub moved_path: Option<PathBuf>,
    pub moved: bool,
}

impl Drop for SavedFile {
    fn drop(&mut self) {
        if !self.moved && self.path.exists() {
            let _ = std::fs::remove_file(&self.path);
        }
    }
}

#[derive(Debug)]
pub struct MultipartData {
    pub fields: HashMap<String, String>,
    pub files: HashMap<String, SavedFile>,
}

#[derive(Debug, Clone)]
pub struct Multipart {
    temp_dir: PathBuf,
}

impl Default for Multipart {
    fn default() -> Self {
        Self::new(Runtime::temp_uploads_dir())
    }
}

impl SavedFile {
    pub fn move_to(&mut self, dest: &PathBuf, replace: bool) -> ResultError<&Self> {
        if dest.exists() {
            if !replace {
                return Err(Error::file_exists(format!(
                    "File already exists: {}",
                    dest.display()
                )));
            }
            if dest.is_dir() {
                return Err(Error::is_a_directory(format!(
                    "Destination is a directory: {}",
                    dest.display()
                )));
            }
        }
        std::fs::rename(self.path.as_path(), dest).map_err(|e| Error::from_io_error(e))?;
        self.moved = true; // mark as move
        self.moved_path = Some(dest.to_path_buf());
        Ok(self)
    }
    pub fn save_to(&self, dest: &PathBuf) -> ResultError<&Self> {
        std::fs::copy(self.path.as_path(), dest).map_err(|e| Error::from_io_error(e))?;
        Ok(self)
    }
}

pub struct CleanUpMetaData {
    pub temp_dir: PathBuf,
    pub start: Instant,
    pub end: Instant,
    pub count: usize,
}

pub struct DayBased {
    pub day: i32,
    pub path: PathBuf,
}

impl DayBased {
    pub fn is_expired(&self) -> bool {
        let now = chrono::Utc::now();
        self.day < now.day() as i32
    }
}

pub struct MonthBased {
    pub month: i32,
    pub days: Vec<DayBased>,
    pub invalid: Vec<PathBuf>,
}

impl MonthBased {
    pub fn is_expired(&self) -> bool {
        let now = chrono::Utc::now();
        self.month < now.month() as i32
    }

    pub fn find_expires(&self) -> Vec<&DayBased> {
        self
            .days
            .iter()
            .clone()
            .filter(|y| y.is_expired())
            .collect::<Vec<&DayBased>>()
    }
}

pub struct YearBased {
    pub year: i32,
    pub months: Vec<MonthBased>,
    pub invalid: Vec<PathBuf>,
}
impl YearBased {
    pub fn is_expired(&self) -> bool {
        let now = chrono::Utc::now();
        self.year < now.year()
    }
    pub fn find_expires(&self) -> Vec<&MonthBased> {
        self
            .months
            .iter()
            .clone()
            .filter(|y| y.is_expired())
            .collect::<Vec<&MonthBased>>()
    }
}

pub struct ScanDirVisitor {
    pub years: Vec<YearBased>,
    pub global_invalid: Vec<PathBuf>,
}

impl ScanDirVisitor {
    pub fn find_expires(&self) -> Vec<&YearBased> {
         self
            .years
            .iter()
            .clone()
            .filter(|y| y.is_expired())
            .collect::<Vec<&YearBased>>()
    }
}

static CLEANUP_QUEUE: LazyLock<DashMap<String, Instant>> = LazyLock::new(|| DashMap::new());
static LAST_CLEANUP: LazyLock<HashMap<String, CleanUpMetaData>> = LazyLock::new(|| HashMap::new());
// expires is one day

impl Multipart {
    pub fn new(temp_dir: PathBuf) -> Self {
        Self { temp_dir }
    }

    pub fn temp_dir(&self) -> &PathBuf {
        &self.temp_dir
    }

    fn rm_recursively(&self, path: &PathBuf) -> ResultError<()> {
        // safe check to make sure only
        if path.starts_with(self.temp_dir()) {
            return Ok(()); // stop
        }
        if path.is_dir() {
            for entry in std::fs::read_dir(path)? {
                let entry = entry?;
                let path = entry.path();
                self.rm_recursively(&path)?;
            }
        }
        std::fs::remove_dir_all(path).map_err(|e| Error::from_io_error(e))?;
        Ok(())
    }

    pub fn scan_visitor(&self) -> ResultError<ScanDirVisitor> {
        let mut years = Vec::new();
        let mut global_invalid = Vec::new();
        if !self.temp_dir.exists() {
            return Ok(ScanDirVisitor {
                years,
                global_invalid,
            });
        }
        for y_entry in std::fs::read_dir(&self.temp_dir).map_err(Error::from_io_error)? {
            let y_path = y_entry.map_err(Error::from_io_error)?.path();
            let y_name = y_path.file_name().and_then(|n| n.to_str()).unwrap_or("");
            // check if year
            if let Ok(year_num) = y_name.parse::<i32>() {
                let mut current_year = YearBased {
                    year: year_num,
                    months: Vec::new(),
                    invalid: Vec::new(),
                };
                // Scan Bulan
                for m_entry in std::fs::read_dir(&y_path).map_err(Error::from_io_error)? {
                    let m_path = m_entry.map_err(Error::from_io_error)?.path();
                    let m_name = m_path.file_name().and_then(|n| n.to_str()).unwrap_or("");

                    if let Ok(month_num) = m_name.parse::<i32>() {
                        let mut current_month = MonthBased {
                            month: month_num,
                            days: Vec::new(),
                            invalid: Vec::new(),
                        };

                        // Scan Hari
                        for d_entry in std::fs::read_dir(&m_path).map_err(Error::from_io_error)? {
                            let d_path = d_entry.map_err(Error::from_io_error)?.path();
                            let d_name = d_path.file_name().and_then(|n| n.to_str()).unwrap_or("");

                            if let Ok(day_num) = d_name.parse::<i32>() {
                                current_month.days.push(DayBased {
                                    day: day_num,
                                    path: d_path,
                                });
                            } else {
                                current_month.invalid.push(d_path);
                            }
                        }
                        current_year.months.push(current_month);
                    } else {
                        current_year.invalid.push(m_path);
                    }
                }
                years.push(current_year);
            } else {
                global_invalid.push(y_path);
            }
        }

        Ok(ScanDirVisitor {
            years,
            global_invalid,
        })
    }

    fn generate_folder_name(&self) -> String {
        let date = chrono::Utc::now();
        // make good folder structures
        format!("{}/{}/{}", date.year(), date.month(), date.day())
    }

    fn generate_file_path(&self) -> (PathBuf, PathBuf) {
        // 1 as directory 2 as file
        let temp_dir = self.temp_dir.join(self.generate_folder_name());
        let file_name = format!("{}", Uuid::v7());
        let path = temp_dir.join(file_name);
        (temp_dir, path)
    }

    // pub fn cleanup(&self) -> ResultError<()> {
    //     let path_buff_string = self.temp_dir().to_string_lossy().to_string();
    //     let lock = CLEANUP_QUEUE.lock();
    //     if lock.contains_key(&path_buff_string) {
    //         return Ok(());
    //     }
    // }

    async fn parse(&self, mut payload: ActixMultipart) -> ResultError<MultipartData> {
        let mut fields = HashMap::new();
        let mut files = HashMap::new();
        while let Some(mut field) = payload.try_next().await.map_err(|e| {
            Error::resource_unavailable(format!("Failed to read multipart field: {}", e))
        })? {
            let (field_name, filename) = {
                let cd = field.content_disposition();
                let name = cd
                    .and_then(|c| c.get_name())
                    .unwrap_or_default()
                    .to_string();
                let file_name = cd.and_then(|c| c.get_filename()).map(|s| s.to_string());
                (name, file_name)
            };
            if let Some(filename) = filename {
                let temp_filename = format!("upload_{}", Uuid::v7());
                let (dir_path, temp_path) = self.generate_file_path();
                if !dir_path.exists() {
                    std::fs::create_dir_all(&dir_path).map_err(|e| Error::from_io_error(e))?;
                }
                if !dir_path.is_dir() {
                    return Err(Error::not_a_directory(format!(
                        "Failed to create directory for multipart upload: {}",
                        dir_path.display()
                    )));
                }
                let mut f =
                    std::fs::File::create(&temp_path).map_err(|e| Error::from_io_error(e))?;
                let mut size = 0u64;

                // Streaming chunk
                while let Some(chunk) = field.next().await {
                    let data = chunk.map_err(|e| {
                        Error::conversion_failed(format!(
                            "Failed to read multipart field chunk: {}",
                            e
                        ))
                    })?;
                    size += data.len() as u64;
                    f.write_all(&data).map_err(|e| Error::from_io_error(e))?;
                }

                files.insert(
                    field_name.to_string(),
                    SavedFile {
                        name: filename.to_string(),
                        path: temp_path,
                        size,
                        content_type: field.content_type().map(|ct| ct.to_string()),
                        moved_path: None,
                        moved: false,
                    },
                );
            } else {
                let mut value = Vec::new();
                while let Some(chunk) = field.next().await {
                    let data = chunk.map_err(|e| {
                        Error::conversion_failed(format!(
                            "Failed to read multipart field chunk: {}",
                            e
                        ))
                    })?;
                    value.extend_from_slice(&data);
                }
                fields.insert(
                    field_name.to_string(),
                    String::from_utf8_lossy(&value).to_string(),
                );
            }
        }

        Ok(MultipartData { fields, files })
    }
}
