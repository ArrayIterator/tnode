use crate::cores::generator::uuid::{Uuid, UuidCrateVersion};
use crate::cores::helper::file_info::FileInfo;
use crate::cores::helper::year_month_day::YearMonthDay;
use crate::cores::system::error::{Error, ResultError};
use crate::cores::system::runtime::Runtime;
use fs2::FileExt;
use std::fs::{copy, create_dir_all, remove_file, rename, File, OpenOptions};
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;

pub const FILE_ITEM_EXT: &str = ".file";

#[derive(Debug)]
pub struct DownloadFileSaver {
    uuid: UuidCrateVersion,
    file_name: String,
    year_month_day: YearMonthDay,
    path: PathBuf,
    claiming: Arc<AtomicBool>
}

impl Default for DownloadFileSaver {
    fn default() -> Self {
        Self::from(Uuid::v7())
    }
}

impl Into<UuidCrateVersion> for DownloadFileSaver {
    fn into(self) -> UuidCrateVersion {
        self.uuid.clone()
    }
}

impl From<UuidCrateVersion> for DownloadFileSaver {
    fn from(uuid: UuidCrateVersion) -> Self {
        let file_name = format!("{}{}", uuid.to_string(), FILE_ITEM_EXT);
        let year_month_day = YearMonthDay::default();
        let path = Runtime::cahe_downloads_dir()
            .join(year_month_day.year().to_string())
            .join(year_month_day.month().to_string())
            .join(year_month_day.day().to_string())
            .join(&file_name);
        Self { uuid, file_name, year_month_day, path, claiming: Arc::new(AtomicBool::new(false)) }
    }
}

impl DownloadFileSaver {
    pub fn from_uuid(uuid: UuidCrateVersion) -> Self {
        Self::from(uuid)
    }
    pub fn is_claiming(&self) -> bool {
        self.claiming.load(Ordering::Acquire)
    }
    pub fn exists(&self) -> bool {
        self.path.exists()
    }
    pub fn get_path(&self) -> &PathBuf {
        &self.path
    }
    pub fn get_file_name(&self) -> &str {
        &self.file_name
    }
    pub fn get_uuid(&self) -> &UuidCrateVersion {
        &self.uuid
    }
    pub fn parent_directory(&self) -> Option<&Path> {
        self.path.parent()
    }
    /// This function attempts to claim a file lock on the file represented by `self.path`.
    /// If the file does not exist, it will be created. The function ensures that the parent directory exists before attempting to create or lock the file. If the file is successfully locked, it will execute the provided asynchronous function `func` with the locked file as an argument. The function returns
    /// a `ResultError<R>` which contains the result of the asynchronous function or any errors that occurred during the file locking process.
    /// # Arguments
    /// * `func` - An asynchronous function that takes a `File` as an argument
    /// and returns a `ResultError<R>`. This function will be executed if the file lock is successfully acquired.
    /// # Returns
    /// * `ResultError<R>` - The result of the asynchronous function `func` or
    /// any errors that occurred during the file locking process.
    /// # Errors
    /// * If the parent directory cannot be created, an error will be returned.
    /// * If the file cannot be created or opened, an error will be returned.
    /// * If the file lock cannot be acquired, an error will be returned. If the file did not exist before the lock attempt, it will be removed to prevent orphaned files.
    /// # Example
    /// ```
    /// let file_item = FileItem::from_uuid(Uuid::v7());
    /// let result = file_item.claim(|file| async {
    ///     // Perform operations with the locked file
    ///     Ok(())
    /// }).await;
    /// match result {
    ///     Ok(_) => println!("File lock acquired and function executed successfully."),
    ///     Err(e) => eprintln!("An error occurred: {:?}", e),
    /// }
    pub async fn claim<R, F, Fut>(&self, func: F) -> ResultError<R>
    where
        F: FnOnce(File) -> Fut,
        Fut: Future<Output = ResultError<R>>,
    {
        if self.claiming.swap(true, Ordering::Acquire) {
            return Err(Error::dead_lock("File is already being claimed by another process or thread"));
        }
        let path = self.path.clone();
        struct ClaimGuard(Arc<AtomicBool>);
        impl Drop for ClaimGuard {
            fn drop(&mut self) {
                self.0.store(false, Ordering::Release);
            }
        }
        let _guard = ClaimGuard(self.claiming.clone());
        let file = tokio::task::spawn_blocking(move || -> ResultError<File> {
            if let Some(parent) = path.parent() {
                if !parent.exists() {
                    create_dir_all(parent).map_err(Error::from_io_error)?;
                }
            }
            let exists = path.exists();
            let f = OpenOptions::new()
                .write(true)
                .create(true)
                .open(&path)
                .map_err(Error::from_io_error)?;
            f.lock_exclusive().map_err(|e|{
                if !exists {
                    remove_file(&path).ok();
                }
                Error::from_io_error(e)
            })?;
            Ok(f)
        }).await.map_err(Error::from_error)??;
        func(file).await
    }

    pub fn move_to<P: AsRef<Path>>(&self, target: P) -> ResultError<()> {
        rename(&self.path, target).map_err(Error::from_io_error)
    }
    pub fn copy_to<P: AsRef<Path>>(&self, target: P) -> ResultError<u64> {
       copy(&self.path, target).map_err(Error::from_io_error)
    }
}

impl Into<FileInfo> for DownloadFileSaver {
    fn into(self) -> FileInfo {
        FileInfo::new(&self.path)
    }
}

impl Into<PathBuf> for DownloadFileSaver {
    fn into(self) -> PathBuf {
        self.path.to_path_buf()
    }
}
