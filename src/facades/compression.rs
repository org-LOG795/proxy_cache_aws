use std::io::prelude::*;
use flate2::Compression;
use flate2::write::GzEncoder;
use opentelemetry::trace::{Tracer};
use tokio::time::{Duration, sleep};

pub async fn compress(bytes: Vec<u8>, tracer: &impl Tracer) -> Vec<u8>
{
 

    let _span = tracer.start("compress");
    sleep(Duration::from_secs(4)).await; // Sleep for 2 seconds
    let mut e = GzEncoder::new(Vec::new(), Compression::default());
    e.write_all(&bytes).unwrap();

    match e.finish() {
        Ok(compressed_bytes) => compressed_bytes,
        Err(_) => panic!("Failed to compress")
    }
}

#[cfg(test)]
mod compression_test {
    use super::*;
    use std::fs;
    use flate2::read::GzDecoder;
    use opentelemetry::trace::noop::NoopTracer;

    //This is only built when we run the unit tests
    fn decompress(bytes: Vec<u8>) -> Vec<u8>
    {
        let mut d = GzDecoder::new(&bytes[..]);
        let mut decompressed = Vec::new();
        d.read_to_end(&mut decompressed).unwrap();

        decompressed
    }
    #[tokio::test]
    async fn compress_valid_file() {
        let contents = fs::read("C:\\Users\\carl-\\Documents\\GTI795\\proxy_cache_aws\\test_files\\poem.txt").expect("NO FILE FOUND");
        
        let tracer = NoopTracer::new();
        let compressed = compress(contents.clone(), &tracer).await;
        let decompressed = decompress(compressed.clone());
        assert!(compressed.len() < contents.len());
        assert_eq!(contents, decompressed);
        println!("completed");
    }    
}