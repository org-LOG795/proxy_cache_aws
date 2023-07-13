use std::io::prelude::*;
use flate2::Compression;
use flate2::write::GzEncoder;
use tokio::time::sleep;
use tracing::{info, span, Level};
use std::time::Duration;

pub async fn compress(bytes: Vec<u8>) -> Vec<u8> {
    let span = span!(Level::INFO, "compress");
    let _enter = span.enter();

    info!("Compressing {} bytes of data", bytes.len());

    let mut e = GzEncoder::new(Vec::new(), Compression::default());
    e.write_all(&bytes).unwrap();

    // simulate a 7 second delay
    //sleep(Duration::from_secs(7)).await;

    let compressed_bytes = match e.finish() {
        Ok(compressed_bytes) => compressed_bytes,
        Err(_) => panic!("Failed to compress")
    };

    println!("Compressed dataV2: {:?}", compressed_bytes);
    info!("Compressed data is {:?} bytes", compressed_bytes.len());

    compressed_bytes
}
#[cfg(test)]
mod compression_test {
    use super::*;
    use std::fs;
    use flate2::read::GzDecoder;
    use tokio::test;

    //This is only built when we run the unit tests
    fn decompress(bytes: Vec<u8>) -> Vec<u8>
    {
        let mut d = GzDecoder::new(&bytes[..]);
        let mut decompressed = Vec::new();
        d.read_to_end(&mut decompressed).unwrap();

        decompressed
    }

    #[test]
    async fn compress_valid_file() {
        let contents = fs::read("T:\\Projects\\proxy_cache_aws\\test_files\\poem.txt").expect("NO FILE FOUND");
        let compressed = compress(contents.clone()).await;
        let decompressed = decompress(compressed.clone());
        assert!(compressed.len() < contents.len());
        assert_eq!(contents, decompressed);
        println!("completed");
    }
}