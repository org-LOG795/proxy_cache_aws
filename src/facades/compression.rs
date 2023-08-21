use std::io::prelude::*;
use flate2::Compression;
use flate2::write::GzEncoder;
use flate2::read::GzDecoder;
use prometheus::{register_histogram_vec, register_int_counter_vec, register_int_counter, HistogramVec, IntCounterVec, IntCounter};
use lazy_static::lazy_static;


lazy_static! {
    static ref COMPRESSION_DURATION: HistogramVec = register_histogram_vec!(
        "compression_duration_seconds",
        "Time taken to compress or decompress data",
        &["operation"]
    )
    .unwrap();
    static ref COMPRESSION_COUNT: IntCounterVec = register_int_counter_vec!(
        "compression_count_total",
        "Number of compression or decompression operations performed",
        &["operation"]
    )
    .unwrap();
    static ref FILE_SIZE: IntCounter = register_int_counter!(
        "file_size_bytes",
        "Size of received files in bytes"
    )
    .unwrap();
}

pub fn gzip_compress(bytes: Vec<u8>) -> Result<Vec<u8>, String> {
    // let timer = COMPRESSION_DURATION
    //     .with_label_values(&["compress"])
    //     .start_timer();
    // COMPRESSION_COUNT.with_label_values(&["compress"]).inc();
    // FILE_SIZE.inc_by(bytes.len() as u64);

    let mut encoder = GzEncoder::new(Vec::new(), Compression::default());
    encoder.write_all(&bytes).map_err(|op| op.to_string())?;
    let result = encoder.finish().map_err(|op| op.to_string());

    //timer.observe_duration();
    result
}


//This is only built when we run the unit tests
pub fn gzip_decompress(bytes: Vec<u8>) -> Result<Vec<u8>, String> {
    let timer = COMPRESSION_DURATION
        .with_label_values(&["decompress"])
        .start_timer();
    COMPRESSION_COUNT.with_label_values(&["decompress"]).inc();
    FILE_SIZE.inc_by(bytes.len() as u64);

    let mut d = GzDecoder::new(&bytes[..]);
    let mut decompressed = Vec::new();
    let result = d
        .read_to_end(&mut decompressed)
        .map(|_| decompressed)
        .map_err(|err| err.to_string());

    timer.observe_duration();
    result
}

#[cfg(test)]
mod compression_test {
    use super::*;
    use std::{fs, path::Path};
    
    fn load_test_file() -> Result<Vec<u8>, String> {
        let path = Path::new("test/poem.txt");
        fs::read(path).map_err(|e| e.to_string())
    }   

    #[test]
    fn compress_valid_file() {
        let contents = load_test_file();
        assert!(contents.is_ok());
        let contents = contents.unwrap();

        let compressed = gzip_compress(contents.clone());
        assert!(compressed.is_ok());
        let compressed = compressed.unwrap();

        let decompressed = gzip_decompress(compressed.clone());
        assert!(decompressed.is_ok());
        let decompressed = decompressed.unwrap();

        //Assert the amount of bytes for compressed file is less
        assert!(compressed.len() < contents.len());
        //Assert that the compression is reversible
        assert_eq!(contents, decompressed);
    }
}