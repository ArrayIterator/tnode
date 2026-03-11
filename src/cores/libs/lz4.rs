use crate::cores::system::error::{Error, ResultError};
use lz4_flex::frame::{FrameDecoder, FrameEncoder};
use std::io::{Read, Write};

pub struct Lz4;

impl Lz4 {
    pub fn compress<T: AsRef<[u8]>>(data: T) -> ResultError<Vec<u8>> {
        let data = data.as_ref();
        let mut encoder = FrameEncoder::new(Vec::new());
        encoder.write_all(data).map_err(Error::from_io_error)?;
        let compressed_data = encoder.finish().map_err(|e|Error::parse_error(e.to_string()))?;
        Ok(compressed_data)
    }

    pub fn decompress<T: AsRef<[u8]>>(compressed_data: T) -> ResultError<Vec<u8>> {
        let compressed_data = compressed_data.as_ref();
        let mut decoder = FrameDecoder::new(compressed_data);
        let mut decompressed_data = Vec::new();
        decoder
            .read_to_end(&mut decompressed_data)
            .map_err(Error::from_io_error)?;
        Ok(decompressed_data)
    }

    pub fn stream_encoder<W: Write>(writer: W) -> FrameEncoder<W> {
        FrameEncoder::new(writer)
    }

    pub fn stream_decoder<R: Read>(reader: R) -> FrameDecoder<R> {
        FrameDecoder::new(reader)
    }
}
