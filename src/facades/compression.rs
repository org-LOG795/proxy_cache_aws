use std::io::prelude::*;
use flate2::Compression;
use flate2::write::GzEncoder;

pub fn gzip_compress(bytes: Vec<u8>) -> Result<Vec<u8>, String>
{
    let mut e = GzEncoder::new(Vec::new(), Compression::default());
    e.write_all(&bytes).and_then( |_| e.finish()).map_err(|op| op.to_string())
}

#[cfg(test)]
mod compression_test {
    use super::*;
    use std::{fs, path::Path};
    use flate2::read::GzDecoder;

    //This is only built when we run the unit tests
    fn gzip_decompress(bytes: Vec<u8>) -> Result<Vec<u8>, String>
    {
        let mut d = GzDecoder::new(&bytes[..]);
        let mut decompressed = Vec::new();
        d.read_to_end(&mut decompressed).map(|_| decompressed).map_err(|err| err.to_string())
    }

    fn load_test_file() -> Result<Vec<u8>, String> {
        let path = Path::new("test_files/poem.txt");
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