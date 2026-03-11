use crate::cores::system::error::{Error, ResultError};
use std::fs::File;
use std::io::{Seek, Write};
use std::path::Path;
use walkdir::WalkDir;
use zip::write::FileOptions;
use zip::{ZipArchive, ZipWriter};

pub struct Zip;

impl Zip {
    pub fn archive_reader<Source: AsRef<Path>>(
        src: Source,
    ) -> ResultError<ZipArchive<File>> {
        let file = File::open(src).map_err(Error::from_io_error)?;
        ZipArchive::new(file).map_err(|e| Error::parse_error(e.to_string()))
    }

    pub fn archive_writer<W: Write + Seek>(
        target: W,
    ) -> ZipWriter<W> {
        ZipWriter::new(target)
    }

    pub fn extract<Source: AsRef<Path>, Target: AsRef<Path>>(
        src: Source,
        folder_dest: Target,
    ) -> ResultError<()> {
        let mut archive = Self::archive_reader(src)?;
        let dest = folder_dest.as_ref();

        for i in 0..archive.len() {
            let mut file = archive.by_index(i).map_err(|e| Error::parse_error(e.to_string()))?;
            let outpath = match file.enclosed_name() {
                Some(path) => dest.join(path),
                None => continue,
            };

            if file.name().ends_with('/') {
                std::fs::create_dir_all(&outpath).map_err(Error::from_io_error)?;
            } else {
                if let Some(p) = outpath.parent() {
                    if !p.exists() {
                        std::fs::create_dir_all(p).map_err(Error::from_io_error)?;
                    }
                }
                let mut outfile = File::create(&outpath).map_err(Error::from_io_error)?;
                std::io::copy(&mut file, &mut outfile).map_err(Error::from_io_error)?;
            }
        }
        Ok(())
    }

    pub fn create_from_dir<Source: AsRef<Path>, Target: AsRef<Path>>(
        src_dir: Source,
        dest_file: Target,
    ) -> ResultError<()> {
        let file = File::create(dest_file).map_err(Error::from_io_error)?;
        let mut zip = Self::archive_writer(file);

        let options: FileOptions<()> = FileOptions::default()
            .compression_method(zip::CompressionMethod::Deflated)
            .unix_permissions(0o755);
        let walk = WalkDir::new(&src_dir);

        for entry in walk.into_iter().filter_map(|e| e.ok()) {
            let path = entry.path();
            let name = path.strip_prefix(&src_dir)
                .map_err(|e| Error::parse_error(e.to_string()))?;
            if path.is_file() {
                zip.start_file(name.to_string_lossy(), options)
                    .map_err(|e| Error::parse_error(e.to_string()))?;

                let mut f = File::open(path).map_err(Error::from_io_error)?;
                std::io::copy(&mut f, &mut zip).map_err(Error::from_io_error)?;
            } else if !name.as_os_str().is_empty() {
                zip.add_directory(name.to_string_lossy(), options)
                    .map_err(|e| Error::parse_error(e.to_string()))?;
            }
        }

        zip.finish().map_err(|e| Error::parse_error(e.to_string()))?;
        Ok(())
    }
}