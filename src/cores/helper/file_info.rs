use crate::cores::helper::user::UserDetail;
use crate::cores::system::error::{Error, ResultError};
use nix::unistd::{Gid, Group, Uid, User};
use std::fmt::Display;
use std::fs::{Metadata, OpenOptions};
use std::ops::Deref;
use std::os::unix::fs::{chown, FileTypeExt, MetadataExt, PermissionsExt};
use std::path::{Path, PathBuf};
use std::time::{SystemTime, UNIX_EPOCH};
use std::{env, fs};

#[derive(Debug)]
pub enum FileType {
    File,
    Directory,
    Link,
    BlockDevice,
    Socket,
    CharacterDevice,
    Fifo,
    Unknown,
}

#[derive(Debug, Clone)]
pub struct FileInfo {
    path: PathBuf,
    init_cwd: PathBuf,
    path_is_absolute: bool,
}

/// FileInfo provides various methods to get information about a file or directory.
/// # Example:
/// ```rust
/// use crate::cores::system::file_info::{FileInfo, FileType};
/// let file_info = FileInfo::new("path/to/file.txt");
/// if file_info.is_exists() {
///     println!("File exists!");
///     if file_info.is_file() {
///         println!("It's a file.");
///     }
///     if file_info.is_executable() {
///         println!("It's executable.");
///     }
/// }
/// ```
impl FileInfo {
    /// Create a new FileInfo instance from a given path.
    /// # Example:
    /// ```rust
    /// use crate::cores::system::file_info::FileInfo;
    /// let file_info = FileInfo::new("path/to/file.txt");
    /// ```
    pub fn new<T: AsRef<Path>>(path: T) -> Self {
        let path = path.as_ref();
        Self {
            path: path.to_path_buf(),
            path_is_absolute: path.is_absolute(),
            init_cwd: env::current_dir().unwrap_or_default(),
        }
    }

    /// Internal method to read standard metadata.
    fn _read_standard_meta(&self, t: FileType) -> bool {
        match self.metadata() {
            Ok(metadata) => match t {
                FileType::File => metadata.is_file(),
                FileType::Directory => metadata.is_dir(),
                FileType::Link => metadata.is_symlink(),
                _ => false,
            },
            Err(_) => false,
        }
    }

    /// Internal method to read symlink metadata.
    fn _read_symlink_meta(&self, t: FileType) -> bool {
        match self.symlink_metadata() {
            Ok(metadata) => match t {
                FileType::File => metadata.is_file(),
                FileType::Directory => metadata.is_dir(),
                FileType::Link => metadata.is_symlink(),
                _ => {
                    let ft = metadata.file_type();
                    #[cfg(unix)]
                    {
                        return match t {
                            FileType::BlockDevice => ft.is_block_device(),
                            FileType::CharacterDevice => ft.is_char_device(),
                            FileType::Socket => ft.is_socket(),
                            FileType::Fifo => ft.is_fifo(),
                            _ => false,
                        };
                    }
                    #[cfg(windows)]
                    false
                }
            },
            Err(_) => false,
        }
    }

    /// Get the standard metadata of the file or directory.
    /// # Example:
    /// ```rust
    /// use crate::cores::system::file_info::FileInfo;
    /// let file_info = FileInfo::new("path/to/file.txt");
    /// match file_info.metadata() {
    ///     Ok(metadata) => println!("File size: {}", metadata.len()),
    ///     Err(e) => println!("Error getting metadata: {}", e),
    /// }
    /// ```
    pub fn metadata(&self) -> ResultError<Metadata> {
        self.path.metadata().map_err(Error::from_error)
    }

    /// Get the symlink metadata of the file or directory.
    /// # Example:
    /// ```rust
    /// use crate::cores::system::file_info::FileInfo;
    /// let file_info = FileInfo::new("path/to/symlink");
    /// match file_info.symlink_metadata() {
    ///     Ok(metadata) => println!("Symlink size: {}", metadata.len()),
    ///     Err(e) => println!("Error getting symlink metadata: {}", e),
    /// }
    /// ```
    pub fn symlink_metadata(&self) -> ResultError<Metadata> {
        self.path.symlink_metadata().map_err(Error::from_error)
    }

    /// Check if the file or directory exists.
    /// # Example:
    /// ```rust
    /// use crate::cores::system::file_info::FileInfo;
    /// let file_info = FileInfo::new("path/to/file.txt");
    /// if file_info.is_exists() {
    ///     println!("File exists!");
    /// } else {
    ///     println!("File does not exist.");
    /// }
    /// ```
    pub fn is_exists(&self) -> bool {
        self.path.try_exists().unwrap_or_else(|_| false)
    }
    /// Get the type of the file or directory.
    /// # Example:
    /// ```rust
    /// use crate::cores::system::file_info::{FileInfo, FileType};
    /// let file_info = FileInfo::new("path/to/file.txt");
    /// match file_info.get_type() {
    ///     FileType::File => println!("It's a file."),
    ///    FileType::Directory => println!("It's a directory."),
    ///    FileType::Link => println!("It's a symlink."),
    ///   _ => println!("Unknown file type."),
    /// }
    /// ```
    pub fn get_type(&self) -> FileType {
        match self.symlink_metadata() {
            Ok(metadata) => {
                let ft = metadata.file_type();
                // Standard cross-platform checks
                if ft.is_dir() {
                    return FileType::Directory;
                }
                if ft.is_file() {
                    return FileType::File;
                }
                if ft.is_symlink() {
                    return FileType::Link;
                }

                // Unix-specific extensions
                #[cfg(unix)]
                {
                    if ft.is_socket() {
                        return FileType::Socket;
                    }
                    if ft.is_block_device() {
                        return FileType::BlockDevice;
                    }
                    if ft.is_char_device() {
                        return FileType::CharacterDevice;
                    }
                    if ft.is_fifo() {
                        return FileType::Fifo;
                    }
                }

                FileType::Unknown
            }
            Err(_) => FileType::Unknown,
        }
    }

    /// Get the canonicalized absolute path (realpath).
    /// # Example:
    /// ```rust
    /// use crate::cores::system::file_info::FileInfo;
    /// let file_info = FileInfo::new("path/to/file.txt");
    /// match file_info.realpath() {
    ///     Some(real_path) => println!("Real path: {}", real_path.display()),
    ///     None => println!("Could not resolve real path."),
    /// }
    /// ```
    pub fn realpath(&self) -> Option<PathBuf> {
        match self.path.canonicalize() {
            Ok(p) => Some(p),
            Err(_) => None,
        }
    }

    /// Check if the path is absolute.
    /// # Example:
    /// ```rust
    /// use crate::cores::system::file_info::FileInfo;
    /// let file_info = FileInfo::new("/absolute/path/to/file.txt");
    /// if file_info.is_absolute() {
    ///     println!("The path is absolute.");
    /// } else {
    ///     println!("The path is not absolute.");
    /// }
    /// ```
    pub fn is_absolute(&self) -> bool {
        self.path_is_absolute
    }

    /// Check if the path is relative.
    /// # Example:
    /// ```rust
    /// use crate::cores::system::file_info::FileInfo;
    /// let file_info = FileInfo::new("relative/path/to/file.txt");
    /// if file_info.is_relative() {
    ///     println!("The path is relative.");
    /// } else {
    ///     println!("The path is not relative.");
    /// }
    pub fn is_relative(&self) -> bool {
        !self.path_is_absolute
    }

    /// Check if the path is a directory.
    /// # Example:
    /// ```rust
    /// use crate::cores::system::file_info::FileInfo;
    /// let file_info = FileInfo::new("path/to/directory");
    /// if file_info.is_dir() {
    ///     println!("It's a directory.");
    /// } else {
    ///     println!("It's not a directory.");
    /// }
    /// ```
    pub fn is_dir(&self) -> bool {
        self.path.is_dir()
    }

    /// Check if the path is a file.
    /// # Example:
    /// ```rust
    /// use crate::cores::system::file_info::FileInfo;
    /// let file_info = FileInfo::new("path/to/file.txt");
    /// if file_info.is_file() {
    ///     println!("It's a file.");
    /// } else {
    ///     println!("It's not a file.");
    /// }
    /// ```
    pub fn is_file(&self) -> bool {
        self.path.is_file()
    }

    /// Check if the path is a symbolic link.
    /// # Example:
    /// ```rust
    /// use crate::cores::system::file_info::FileInfo;
    /// let file_info = FileInfo::new("path/to/symlink");
    /// if file_info.is_link() {
    ///     println!("It's a symbolic link.");
    /// } else {
    ///     println!("It's not a symbolic link.");
    /// }
    /// ```
    pub fn is_link(&self) -> bool {
        self.path.is_symlink()
    }

    /// Check if the path is a regular file.
    /// # Example:
    /// ```rust
    /// use crate::cores::system::file_info::FileInfo;
    /// let file_info = FileInfo::new("path/to/file.txt");
    /// if file_info.is_regular_file() {
    ///     println!("It's a regular file.");
    /// } else {
    ///     println!("It's not a regular file.");
    /// }
    pub fn is_block_device(&self) -> bool {
        self._read_symlink_meta(FileType::BlockDevice)
    }

    /// Check if the path is a directory.
    /// # Example:
    /// ```rust
    /// use crate::cores::system::file_info::FileInfo;
    /// let file_info = FileInfo::new("path/to/directory");
    /// if file_info.is_directory() {
    ///     println!("It's a directory.");
    /// } else {
    ///     println!("It's not a directory.");
    /// }
    pub fn is_character_device(&self) -> bool {
        self._read_symlink_meta(FileType::CharacterDevice)
    }

    /// Check if the path is a named pipe (FIFO).
    /// # Example:
    /// ```rust
    /// use crate::cores::system::file_info::FileInfo;
    /// let file_info = FileInfo::new("path/to/fifo");
    /// if file_info.is_fifo() {
    ///     println!("It's a named pipe (FIFO).");
    /// } else {
    ///     println!("It's not a named pipe (FIFO).");
    /// }
    pub fn is_fifo(&self) -> bool {
        self._read_symlink_meta(FileType::Fifo)
    }

    /// Check if the path is a Unix domain socket.
    /// # Example:
    /// ```rust
    /// use crate::cores::system::file_info::FileInfo;
    /// let file_info = FileInfo::new("path/to/socket");
    /// if file_info.is_socket() {
    ///     println!("It's a Unix domain socket.");
    /// } else {
    ///     println!("It's not a Unix domain socket.");
    /// }
    pub fn is_socket(&self) -> bool {
        self._read_symlink_meta(FileType::Socket)
    }

    /// Check if the path is a symbolic link to a directory.
    /// # Example:
    /// ```rust
    /// use crate::cores::system::file_info::FileInfo;
    /// let file_info = FileInfo::new("path/to/symlink");
    /// if file_info.is_symlink_dir() {
    ///     println!("It's a symbolic link to a directory.");
    /// } else {
    ///     println!("It's not a symbolic link to a directory.");
    /// }
    pub fn is_executable(&self) -> bool {
        match self.metadata() {
            Ok(metadata) => {
                #[cfg(unix)]
                {
                    // On Unix, we check the mode bits for an owner, group, or others
                    let mode = metadata.permissions().mode();
                    mode & 0o111 != 0
                }
                // #[cfg(windows)]
                // {
                //     // On Windows, Rust doesn't have a built-in 'executable' bit.
                //     // Usually, we check the file extension.
                //     if let Some(ext) = self.path.extension() {
                //         let ext = ext.to_string_lossy().to_lowercase();
                //         matches!(ext.as_str(), "exe" | "bat" | "cmd" | "ps1")
                //     } else {
                //         false
                //     }
                // }
                // false
            }
            Err(_) => false,
        }
    }

    /// Check if the path is a symbolic link to a regular file.
    /// # Example:
    /// ```rust
    /// use crate::cores::system::file_info::FileInfo;
    /// let file_info = FileInfo::new("path/to/symlink");
    /// if file_info.is_symlink_file() {
    ///     println!("It's a symbolic link to a regular file.");
    /// } else {
    ///     println!("It's not a symbolic link to a regular file.");
    /// }
    pub fn is_readable(&self) -> bool {
        if self.path.is_dir() {
            return fs::read_dir(&self.path).is_ok();
        }
        OpenOptions::new().read(true).open(&self.path).is_ok()
    }

    /// Check if the path is a symbolic link to a directory.
    /// # Example:
    /// ```rust
    /// use crate::cores::system::file_info::FileInfo;
    /// let file_info = FileInfo::new("path/to/symlink");
    /// if file_info.is_symlink_dir() {
    ///     println!("It's a symbolic link to a directory.");
    /// } else {
    ///     println!("It's not a symbolic link to a directory.");
    /// }
    pub fn is_writable(&self) -> bool {
        if self.path.is_dir() {
            let nanos = SystemTime::now()
                .duration_since(UNIX_EPOCH)
                .map(|d| d.as_nanos())
                .unwrap_or(0);
            let temp_file_name = format!(".__test_{}", nanos);
            return match OpenOptions::new()
                .write(true)
                .create(true)
                .open(&temp_file_name)
            {
                Ok(_) => {
                    let _ = fs::remove_file(temp_file_name);
                    true
                }
                Err(_) => false,
            };
        }
        OpenOptions::new()
            .write(true)
            .append(true)
            .open(&self.path)
            .is_ok()
    }

    /// Get file access times.
    /// # Example:
    /// ```rust
    /// use crate::cores::system::file_info::FileInfo;
    /// let file_info = FileInfo::new("path/to/file.txt");
    /// match file_info.get_access_times() {
    ///     Ok(times) => println!("File access times: {:#?}", times),
    ///     Err(e) => println!("Error getting file access times: {}", e),
    /// }
    /// ```
    pub fn a_time(&self) -> Option<SystemTime> {
        match self.metadata() {
            Ok(metadata) => metadata.accessed().ok(),
            Err(_) => None,
        }
    }

    /// Get file modification times.
    /// # Example:
    /// ```rust
    /// use crate::cores::system::file_info::FileInfo;
    /// let file_info = FileInfo::new("path/to/file.txt");
    /// match file_info.get_modification_times() {
    ///     Ok(times) => println!("File modification times: {:#?}", times),
    ///     Err(e) => println!("Error getting file modification times: {}", e),
    /// }
    /// ```
    pub fn m_time(&self) -> Option<SystemTime> {
        match self.metadata() {
            Ok(metadata) => metadata.modified().ok(),
            Err(_) => None,
        }
    }

    /// Get file creation times.
    /// # Example:
    /// ```rust
    /// use crate::cores::system::file_info::FileInfo;
    /// let file_info = FileInfo::new("path/to/file.txt");
    /// match file_info.get_creation_times() {
    ///     Ok(times) => println!("File creation times: {:#?}", times),
    ///     Err(e) => println!("Error getting file creation times: {}", e),
    /// }
    /// ```
    pub fn c_time(&self) -> Option<SystemTime> {
        match self.metadata() {
            Ok(metadata) => metadata.created().ok(),
            Err(_) => None,
        }
    }

    /// Get file size in bytes.
    /// # Example:
    /// ```rust
    /// use crate::cores::system::file_info::FileInfo;
    /// let file_info = FileInfo::new("path/to/file.txt");
    /// match file_info.get_size() {
    ///     Ok(size) => println!("File size: {} bytes", size),
    ///     Err(e) => println!("Error getting file size: {}", e),
    /// }
    /// ```
    pub fn size(&self) -> Option<u64> {
        match self.metadata() {
            Ok(metadata) => Some(metadata.len()),
            Err(_) => None,
        }
    }

    /// Get file basename.
    /// # Example:
    /// ```rust
    /// use crate::cores::system::file_info::FileInfo;
    /// let file_info = FileInfo::new("path/to/file.txt");
    /// match file_info.basename() {
    ///     Some(name) => println!("File basename: {}", name),
    ///     None => println!("Error getting file basename."),
    /// }
    /// ```
    pub fn basename(&self) -> Option<String> {
        match self.path.file_name() {
            Some(name_osstr) => Some(name_osstr.to_string_lossy().to_string()),
            None => None,
        }
    }

    pub fn get_inode(&self) -> Option<u64> {
        #[cfg(unix)]
        {
            use std::os::unix::fs::MetadataExt;
            match self.metadata() {
                Ok(metadata) => Some(metadata.ino()),
                Err(_) => None,
            }
        }
        #[cfg(not(unix))]
        {
            None
        }
    }

    /// Get a file name without extension.
    /// # Example:
    /// ```rust
    /// use crate::cores::system::file_info::FileInfo;
    /// let file_info = FileInfo::new("path/to/file.txt");
    /// match file_info.name() {
    ///     Some(name) => println!("File name: {}", name),
    ///     None => println!("Error getting file name."),
    /// }
    /// ```
    pub fn filename(&self) -> Option<String> {
        match self.path.file_name() {
            Some(name_osstr) => Some(name_osstr.to_string_lossy().to_string()),
            None => None,
        }
    }

    /// Get file extension.
    /// # Example:
    /// ```rust
    /// use crate::cores::system::file_info::FileInfo;
    /// let file_info = FileInfo::new("path/to/file.txt");
    /// match file_info.extension() {
    ///     Some(ext) => println!("File extension: {}", ext),
    ///     None => println!("Error getting file extension."),
    /// }
    /// ```
    pub fn extension(&self) -> Option<String> {
        match self.path.extension() {
            Some(ext_osstr) => Some(ext_osstr.to_string_lossy().to_string()),
            None => None,
        }
    }

    /// Gt permission bits.
    /// # Example:
    /// ```rust
    /// use crate::cores::system::file_info::FileInfo;
    /// let file_info = FileInfo::new("path/to/file.txt");
    /// match file_info.permissions() {
    ///     Some(perm) => println!("File permissions: {:#o}", perm),
    ///     None => println!("Error getting file permissions."),
    /// }
    /// ```
    pub fn permissions(&self) -> Option<u32> {
        match self.metadata() {
            Ok(metadata) => Some(metadata.permissions().mode()),
            Err(_) => None,
        }
    }

    /// Get owner USER ID.
    /// # Example:
    /// ```rust
    /// use crate::cores::system::file_info::FileInfo;
    /// let file_info = FileInfo::new("path/to/file.txt");
    /// match file_info.uid() {
    ///     Some(uid) => println!("File owner UID: {}", uid),
    ///     None => println!("Error getting file owner UID."),
    /// }
    /// ```
    pub fn uid(&self) -> Option<u32> {
        #[cfg(unix)]
        {
            match self.metadata() {
                Ok(metadata) => Some(metadata.uid()),
                Err(_) => None,
            }
        }
        #[cfg(not(unix))]
        {
            None
        }
    }

    /// Get owner group ID.
    /// # Example:
    /// ```rust
    /// use crate::cores::system::file_info::FileInfo;
    /// let file_info = FileInfo::new("path/to/file.txt");
    /// match file_info.gid() {
    ///     Some(gid) => println!("File owner GID: {}", gid),
    ///     None => println!("Error getting file owner GID."),
    /// }
    /// ```
    pub fn gid(&self) -> Option<u32> {
        #[cfg(unix)]
        {
            match self.metadata() {
                Ok(metadata) => Some(metadata.gid()),
                Err(_) => None,
            }
        }
        #[cfg(not(unix))]
        {
            None
        }
    }

    /// Get file USER.
    /// Returning User struct if the file is owned by a USER.
    /// # Example:
    /// ```rust
    /// use crate::cores::system::file_info::FileInfo;
    /// let file_info = FileInfo::new("path/to/file.txt");
    /// match file_info.USER() {
    ///     Some(USER) => println!("File owner USER: {}", USER.name()),
    ///     None => println!("Error getting file owner USER."),
    /// }
    /// ```
    pub fn owner(&self) -> Option<UserDetail> {
        #[cfg(unix)]
        {
            let user = if let Some(uid) = self.uid() {
                let uid = Uid::from_raw(uid);
                if let Ok(user) = User::from_uid(uid) {
                    user
                } else {
                    None
                }
            } else {
                None
            };
            let group = if let Some(uid) = self.gid() {
                let uid = Gid::from_raw(uid);
                if let Ok(group) = Group::from_gid(uid) {
                    group
                } else {
                    None
                }
            } else {
                None
            };
            if let Some(user) = user
                && let Some(group) = group
            {
                return Some(UserDetail { user, group });
            }
            return None;
        }
        #[cfg(not(unix))]
        {
            None
        }
    }

    pub fn absolute_path(&self) -> PathBuf {
        // try to get a realpath first
        let path = self.realpath();
        if path.is_some() {
            return path.unwrap();
        }
        if self.is_absolute() {
            return self.path.clone();
        }
        self.init_cwd.join(&self.path)
    }

    /// Get the parent directory path.
    /// # Example:
    /// ```rust
    /// use crate::cores::system::file_info::FileInfo;
    /// let file_info = FileInfo::new("path/to/file.txt");
    /// match file_info.parent_dir() {
    ///     Ok(path) => println!("Parent directory path: {}", path.display()),
    ///     Err(e) => println!("Error getting parent directory path: {}", e),
    /// }
    /// ```
    pub fn dirname(&self) -> Option<String> {
        match self.path.parent() {
            Some(parent_path) => Some(parent_path.to_string_lossy().to_string()),
            None => None,
        }
    }

    /// Set file permissions.
    /// # Example:
    /// ```rust
    /// use crate::cores::system::file_info::FileInfo;
    /// let file_info = FileInfo::new("path/to/file.txt");
    /// match file_info.chmod(0o777) {
    ///     Ok(changed) => println!("File permissions changed: {}", changed),
    ///     Err(e) => println!("Error changing file permissions: {}", e),
    /// }
    /// ```
    pub fn chmod(&self, mode: u32) -> ResultError<bool> {
        let meta = self.metadata()?;
        let current = meta.permissions().mode() & 0o777;

        if current == mode {
            return Ok(false); // already correct
        }

        let perm = fs::Permissions::from_mode(mode);
        fs::set_permissions(&self.path, perm).map_err(Error::from_error)?;

        Ok(true)
    }

    /// Set file owner.
    /// # Example:
    /// ```rust
    /// use crate::cores::system::file_info::FileInfo;
    /// let file_info = FileInfo::new("path/to/file.txt");
    /// match file_info.chown(1000, 1000) {
    ///     Ok(changed) => println!("File owner changed: {}", changed),
    ///     Err(e) => println!("Error changing file owner: {}", e),
    /// }
    /// ```
    pub fn chown(&self, uid: u32, gid: u32) -> ResultError<bool> {
        #[cfg(unix)]
        {
            if !self.is_exists() {
                return Ok(false);
            }

            let path_str = self.path.to_str();
            if path_str.is_none() {
                return Ok(false);
            }
            let path_str = path_str.unwrap();
            if path_str.is_empty() {
                return Ok(false);
            }
            let f_uid = self.uid();
            let f_gid = self.gid();
            if !f_uid.is_none()
                && !f_gid.is_none()
                && f_uid.unwrap() == uid
                && f_gid.unwrap() == gid
            {
                return Ok(true); // already correct
            }
            // set ownership
            if let Err(e) = chown(path_str, Some(uid), Some(gid)) {
                return Err(Error::from_error(e));
            }
            Ok(true)
        }
        #[cfg(not(unix))]
        {
            // Chown is not supported on non-Unix systems
            Ok(false)
        }
    }

    /// Unlink / Remove a file or directory.
    /// # Example:
    /// ```rust
    /// use crate::cores::system::file_info::FileInfo;
    /// let file_info = FileInfo::new("path/to/file.txt");
    /// match file_info.unlink() {
    ///     Ok(removed) => println!("File removed: {}", removed),
    ///     Err(e) => println!("Error removing file: {}", e),
    /// }
    /// ```
    pub fn unlink(&self) -> ResultError<bool> {
        if self.is_dir() {
            let res = match fs::remove_dir(&self.path) {
                Ok(_) => Ok(true),
                Err(e) => Err(Error::permission_denied(format!(
                    "Failed to remove directory {}: {}",
                    self, e
                ))),
            };
            return res;
        }
        match fs::remove_file(&self.path) {
            Ok(_) => Ok(true),
            Err(e) => Err(Error::permission_denied(format!(
                "Failed to remove file {}: {}",
                self, e
            ))),
        }
    }

    /// Move a file or directory.
    /// # Example:
    /// ```rust
    /// use crate::cores::system::file_info::FileInfo;
    /// let file_info = FileInfo::new("path/to/file.txt");
    /// match file_info.rename("new_path/to/file.txt") {
    ///     Ok(renamed) => println!("File renamed: {}", renamed),
    ///     Err(e) => println!("Error renaming file: {}", e),
    /// }
    /// ```
    pub fn rename(&self, new_path: &str) -> ResultError<bool> {
        match fs::rename(&self.path, new_path) {
            Ok(_) => Ok(true),
            Err(e) => Err(Error::permission_denied(format!(
                "Failed to rename {} to {}: {}",
                self, new_path, e
            ))),
        }
    }

    /// Get the absolute path from a given path.
    /// # Example:
    /// ```rust
    /// use crate::cores::system::file_info::FileInfo;
    /// match FileInfo::abs("relative/path/to/file.txt") {
    ///     Ok(abs_path) => println!("Absolute path: {}", abs_path.display()),
    ///     Err(e) => println!("Error getting absolute path: {}", e),
    /// }
    /// ```
    pub fn abs<P: AsRef<Path>>(path: P) -> ResultError<PathBuf> {
        let p = path.as_ref();
        if p.is_absolute() {
            Ok(p.to_path_buf())
        } else {
            env::current_dir()
                .map_err(|e| Error::from_error(e))
                .map(|cwd| cwd.join(p))
                .map_err(|e| Error::from(e))
        }
    }

    /// Get the path as a reference to a Path object.
    ///
    /// # Example:
    /// ```rust
    /// use crate::cores::system::file_info::FileInfo;
    /// let file_info = FileInfo::new("path/to/file.txt");
    /// println!("Path: {}", file_info.as_path().display());
    /// ```
    pub fn as_path(&self) -> &Path {
        &self.path.as_path()
    }
}

impl Display for FileInfo {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> Result<(), std::fmt::Error> {
        f.write_str(self.path.as_os_str().to_str().unwrap_or_default())
    }
}
impl Deref for FileInfo {
    type Target = Path;
    fn deref(&self) -> &Self::Target {
        &self.path
    }
}
