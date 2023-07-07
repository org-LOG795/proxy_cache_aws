use std::io::prelude::*;
use flate2::Compression;
use flate2::write::GzEncoder;
use opentelemetry::trace::{Tracer};
use tokio::time::{Duration, sleep};

pub async fn compress(bytes: Vec<u8>, tracer: &impl Tracer) -> Vec<u8>
{

    let _span = tracer.start("compress");

      // Only include the delay code when running tests
      #[cfg(test)]
      {
          sleep(Duration::from_secs(7)).await; // Sleep for 4 seconds
      }

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

    #[tokio::test]
    async fn test_compress_7_secs() {
        let tracer = opentelemetry_jaeger::new_agent_pipeline()
        .with_service_name("test_delay_7_secs")
        .install_simple()
        .expect("Failed to install Jaeger pipeline");

        let bytes = vec![1, 2, 3, 4, 5];
        
        let start = std::time::Instant::now();
        
        let compressed = compress(bytes.clone(), &tracer).await;
        
        let duration = start.elapsed();
        
        assert_eq!(compressed, vec![31, 139, 8, 0, 0, 0, 0, 0, 0, 255, 99, 100, 98, 102, 97, 5, 0, 244, 153, 11, 71, 5, 0, 0, 0]);
        println!("Compression with a delay of 7 seconds took {:?}", duration);
    } 

}