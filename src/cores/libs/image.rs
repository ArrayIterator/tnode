use crate::cores::libs::file_mime_type::FileMimeType;
use imagesize::{ImageError, ImageResult, ImageSize as ImgSize};
use std::io::ErrorKind;
use std::path::Path;
use std::sync::{Arc, OnceLock};

#[derive(Debug, Clone)]
enum ErrorImageType {
    Corrupt,
    Unsupported,
    IoError(ErrorKind, String),
}

impl ErrorImageType {
    fn to_error(&self) -> ImageError {
        match self {
            ErrorImageType::Corrupt => ImageError::CorruptedImage,
            ErrorImageType::Unsupported => ImageError::NotSupported,
            ErrorImageType::IoError(kind, msg) => {
                ImageError::IoError(std::io::Error::new(kind.clone(), msg.clone()))
            }
        }
    }
}

#[derive(Debug, Clone)]
pub enum Percentage {
    Hundred(usize),
    Percent(f64),
}

impl Percentage {
    pub fn as_multiplier(&self) -> f64 {
        match self {
            Percentage::Hundred(i) => *i as f64 / 100.0,
            Percentage::Percent(p) => *p,
        }
    }
}

#[derive(Debug)]
pub struct ImageData {
    file_mime: Arc<FileMimeType>,
    is_image: OnceLock<bool>,
    image_size: OnceLock<Result<ImgSize, ErrorImageType>>,
}

#[derive(Debug)]
pub struct Image {
    image_data: Arc<ImageData>,
}

#[derive(Debug, Clone)]
pub struct ImageSize {
    pub width: usize,
    pub height: usize,
}
impl From<ImgSize> for ImageSize {
    fn from(size: ImgSize) -> Self {
        Self {
            width: size.width,
            height: size.height,
        }
    }
}

#[derive(Debug)]
pub struct ImageSizes {
    pub original: ImageSize,
    pub proportional: ImageSize,
}

impl ImageData {
    pub fn new<T: AsRef<Path>>(data: T) -> Self {
        Self {
            file_mime: Arc::new(FileMimeType::new(data)),
            is_image: OnceLock::new(),
            image_size: OnceLock::new(),
        }
    }
    pub fn file_mime(&self) -> Arc<FileMimeType> {
        self.file_mime.clone()
    }
    pub fn path(&self) -> &Path {
        self.file_mime.path()
    }
    pub fn is_image(&self) -> bool {
        self.is_image
            .get_or_init(|| {
                if let Some(mime) = self.file_mime.mime() {
                    mime.type_() == mime::IMAGE
                } else {
                    false
                }
            })
            .clone()
    }
    pub fn image_size(&self) -> ImageResult<ImgSize> {
        let e = self.image_size.get_or_init(|| {
            if self.is_image() == false {
                return Err(ErrorImageType::Unsupported);
            }
            match imagesize::size(self.file_mime.path()) {
                Ok(e) => Ok(e),
                Err(e) => Err(match e {
                    ImageError::NotSupported => ErrorImageType::Unsupported,
                    ImageError::CorruptedImage => ErrorImageType::Corrupt,
                    ImageError::IoError(e) => ErrorImageType::IoError(e.kind(), e.to_string()),
                }),
            }
        });
        match e {
            Ok(i) => Ok(*i),
            Err(e) => Err(e.to_error()),
        }
    }
}

impl From<ImageData> for Image {
    fn from(data: ImageData) -> Self {
        Self::from_image_data(data)
    }
}

impl Image {
    pub fn new<T: AsRef<Path>>(path: T) -> Self {
        Self::from(ImageData::new(path))
    }

    pub fn from_image_data(data: ImageData) -> Self {
        Self {
            image_data: Arc::new(data),
        }
    }

    pub fn image_data(&self) -> Arc<ImageData> {
        self.image_data.clone()
    }

    pub fn image_size(&self) -> ImageResult<ImgSize> {
        self.image_data().image_size()
    }

    pub fn calculate_proportional_by_width(&self, target_width: usize, size: ImgSize) -> ImageSize {
        let ImgSize {
            width: w,
            height: h,
            ..
        } = size;

        if w == target_width {
            return ImageSize::from(size);
        }

        let scale_factor = target_width as f64 / w as f64;
        let new_height = (h as f64 * scale_factor).round() as usize;
        ImageSize {
            width: target_width,
            height: new_height,
        }
    }

    pub fn calculate_proportional_by_height(
        &self,
        target_height: usize,
        size: ImgSize,
    ) -> ImageSize {
        let ImgSize {
            width: w,
            height: h,
            ..
        } = size;
        if h == target_height {
            return ImageSize::from(size);
        }
        let scale_factor = target_height as f64 / h as f64;
        let new_width = (w as f64 * scale_factor).round() as usize;
        ImageSize {
            width: new_width,
            height: target_height,
        }
    }
    pub fn calculate_fit(&self, target_w: usize, target_h: usize, size: ImgSize) -> ImageSize {
        let ImgSize {
            width: w,
            height: h,
            ..
        } = size;
        let width_scale = target_w as f64 / w as f64;
        let height_scale = target_h as f64 / h as f64;

        let final_scale = width_scale.min(height_scale);
        let new_width = (w as f64 * final_scale).round() as usize;
        let new_height = (h as f64 * final_scale).round() as usize;
        ImageSize {
            width: new_width,
            height: new_height,
        }
    }

    pub fn proportional_size(
        &self,
        width: Option<usize>,
        height: Option<usize>,
    ) -> ImageResult<ImageSizes> {
        let size = self.image_size()?;
        let new_size = match (width, height) {
            (Some(w), Some(h)) => self.calculate_fit(w, h, size),
            (Some(w), None) => self.calculate_proportional_by_width(w, size),
            (None, Some(h)) => self.calculate_proportional_by_height(h, size),
            (None, None) => ImageSize::from(size),
        };
        Ok(ImageSizes {
            original: ImageSize::from(size),
            proportional: new_size,
        })
    }

    pub fn proportional_size_min(&self, min_size: usize) -> ImageResult<ImageSizes> {
        self.proportional_size(Some(min_size), Some(min_size))
    }

    pub fn proportional_percentage(&self, scale: Percentage) -> ImageResult<ImageSizes> {
        let original_size = self.image_size()?;
        let multiplier = scale.as_multiplier();
        Ok(ImageSizes {
            original: ImageSize::from(original_size),
            proportional: ImageSize {
                width: (original_size.width as f64 * multiplier).round() as usize,
                height: (original_size.height as f64 * multiplier).round() as usize,
            },
        })
    }
}
