use mime::Mime;
use std::path::{Path, PathBuf};
use std::sync::OnceLock;

/// Represents a MIME type associated with a file path.
///
/// The `MimeType` struct is used to store and manage the MIME type of a file.
/// It contains the file path and a lazily initialized MIME type.
///
/// # Fields
///
/// * `path` - The file path associated with the MIME type, stored as a `PathBuf`.
/// * `mime` - A lazily initialized `OnceLock` that stores an `Option<Mime>`.
///   The MIME type is determined and cached the first time it is accessed.
///
/// # Usage
///
/// This struct is useful for scenarios where MIME type detection for files
/// is required, avoiding repeated computation by caching the result in a
/// thread-safe manner.
///
/// # Example
///
/// ```rust
/// use std::path::PathBuf;
/// use std::sync::OnceLock;
/// use some_crate::Mime;
/// use your_crate::MimeType;
///
/// let file_path = PathBuf::from("example.txt");
/// let mime_type = MimeType {
///     path: file_path,
///     mime: OnceLock::new(),
/// };
///
/// // Use `mime_type` to determine and access the MIME type.
/// ```
#[derive(Debug)]
pub struct FileMimeType {
    path: PathBuf,
    mime: OnceLock<Option<Mime>>,
}

impl FileMimeType {
    /// Creates a new `MimeType` instance with the specified file path.
    pub fn new<T: AsRef<Path>>(path: T) -> Self {
        Self {
            path: path.as_ref().to_path_buf(),
            mime: OnceLock::new(),
        }
    }
    /// Returns the file path associated with the MIME type.
    pub fn path(&self) -> &Path {
        self.path.as_path()
    }
    /// Returns the MIME type of the file.
    pub fn mime(&self) -> Option<Mime> {
        self.mime
            .get_or_init(|| {
                if !self.path.exists() {
                    return None;
                }
                match infer::get_from_path(self.path.as_path()) {
                    Ok(p) => {
                        if let Some(t) = p {
                            if let Ok(mime) = t.mime_type().parse::<Mime>() {
                                return Some(mime);
                            }
                        }
                        None
                    }
                    Err(_) => None,
                }
            })
            .clone()
    }
}
