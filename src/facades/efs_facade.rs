use std::path::Path;
use tokio::fs;
use std::io::{Write, Read, Seek, SeekFrom};
use std::ops::Range<u64>;
use std::fs::OpenOptions;

pub struct EfsFacade {}


impl<'a> EfsFacade<'a> {
    pub fn new() -> Self {
        EfsFacade {}
    }

    pub async fn write(&self, bytes: &[u8], archive_name: String) -> Result<(), Range<u64>> {

        let mut options = OpenOptions::new();
        options.write(true).create_new(true);

        match options.open(archive_name).await {
            Ok(mut file) => {

                let current_offset = file.seek(SeekFrom::Current(0)).await.unwrap();

                if let Err(err) = file.write_all(bytes).await {
                    println!("Error writing to offset file: {}", err);
                }

                println!("Data written to the offset file");
                
                let new_offset = file.seek(SeekFrom::Current(0)).await.unwrap();
                let offset_range = current_offset..new_offset;

                Ok(offset_range);
            }
            Err(err) => {
                println!("Error opening file: {}", err);
            }
        }
    }

    pub async fn read(&mut self, offset_range: &Range<u64>, file_name: &str) -> Result<(), Vec<u8>> {

        if let Some(start) = offset_range.start {
            if let Some(end) = offset_range.end {
                let mut options = OpenOptions::new();
                let mut file = options.read(true).open(file_name).await?;
                
                file.seek(SeekFrom::Start(start)).await?;
                
                let bytes_range = (end - start) as usize;
                let mut bytes = vec![0u8; bytes_range];
                file.read_exact(&mut bytes).await?;
                
                Ok(bytes);
            } else {
                Err("Error reading the end offset".into())
            }
        } else {
            Err("Error reading the start of the offset".into())
        }
    }
}