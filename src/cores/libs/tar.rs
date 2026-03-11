use crate::cores::system::error::{Error, ResultError};
use flate2::read::GzDecoder;
use flate2::write::GzEncoder;
use flate2::Compression;
use std::fs::File;
use std::io::Write;
use std::path::Path;
use tar::{Archive, Builder};

pub struct Tar;

impl Tar {
    pub fn archive_reader<Source: AsRef<Path>>(
        src: Source,
    ) -> ResultError<Archive<GzDecoder<File>>> {
        let tar_gz = File::open(src).map_err(Error::from_io_error)?;
        let enc = GzDecoder::new(tar_gz);
        Ok(Archive::new(enc))
    }

    pub fn archive_writer<W: Write>(
        target: W,
        compression: Compression,
    ) -> ResultError<Builder<GzEncoder<W>>> {
        let enc = GzEncoder::new(target, compression);
        let tar = Builder::new(enc);
        Ok(tar)
    }

    pub fn extract<Source: AsRef<Path>, Target: AsRef<Path>>(
        src: Source,
        folder_dest: Target,
    ) -> ResultError<()> {
        let mut archive = Self::archive_reader(src)?;
        archive.unpack(folder_dest)?;
        Ok(())
    }

    pub fn create_from_dir<Source: AsRef<Path>, Target: AsRef<Path>>(
        src_dir: &str,
        dest_file: Target,
    ) -> ResultError<()> {
        let tar_gz = File::create(dest_file)?;
        let mut tar = Self::archive_writer(tar_gz, Compression::default())?;
        tar.append_dir_all("../../..", src_dir)
            .map_err(Error::from_io_error)?;
        tar.finish().map_err(Error::from_io_error)?;
        Ok(())
    }
}
