use chrono::prelude::*;
use serde::{Deserialize, Serialize};
use std::error::Error;
use std::ops::Range;
use std::path::Path;
use tokio::fs::{self, File, OpenOptions};
use tokio::io::{self, AsyncReadExt, AsyncSeekExt, AsyncWriteExt, SeekFrom};

pub struct EfsFacade {}

#[derive(Debug, Serialize, Deserialize)]
struct Metadata {
    creation_date: String,
    range: String,
    compression: String,
}

impl EfsFacade {
    pub fn new() -> Self {
        EfsFacade {}
    }

    async fn create_dir(dir_name: &str) -> Result<(), tokio::io::Error> {
        let path = Path::new(dir_name);

        match fs::create_dir(path).await {
            Ok(_) => {
                println!("Directory created: {:#?}", path);
                Ok(())
            }
            Err(err) if err.kind() == tokio::io::ErrorKind::AlreadyExists => {
                println!("Directory already exists: {:#?}", path);
                Ok(())
            }
            Err(err) => Err(err),
        }
    }

    async fn write_to_offset_file(
        offset_file_path: String,
        bytes_lenght: usize,
    ) -> Result<Range<u64>, Box<dyn Error>> {
        let mut options = OpenOptions::new();
        options.write(true).create(true).read(true);

        let mut starting_offset: u64 = 0;
        let mut ending_offset: u64 = 0;

        match options.open(offset_file_path).await {
            Ok(mut offset_file) => {
                if offset_file.metadata().await.unwrap().len() != 0 {
                    // We assume if it's not 0, there's 8 bytes
                    let mut buffer = [0_u8; 8];

                    offset_file.read(&mut buffer).await?;

                    starting_offset = u64::from_ne_bytes(buffer);

                    offset_file.seek(SeekFrom::Start(0)).await?;
                }
                ending_offset = starting_offset + bytes_lenght as u64;
                offset_file.write(&ending_offset.to_ne_bytes()).await?;
                println!("Offset range written to the offset file");
            }
            Err(e) => {
                println!("Error opening offset file: {}", e);
                return Err(Box::new(e));
            }
        }

        Ok(starting_offset..ending_offset)
    }

    async fn create_file(file_path: String, bytes: &[u8]) -> Result<(), Box<dyn Error>> {
        let mut options = OpenOptions::new();
        options.write(true).create_new(true);

        match options.open(file_path).await {
            Ok(mut file) => {
                file.write_all(bytes).await?;
                println!("File created",);
            }

            Err(e) => {
                println!("Error opening file: {}", e);
                return Err(Box::new(e));
            }
        }

        Ok(())
    }

    async fn read_file(file_path: String) -> Result<Vec<u8>, Box<dyn Error>> {
        let mut options = OpenOptions::new();
        options.read(true);
        let mut buffer = Vec::new();

        match options.open(file_path).await {
            Ok(mut file) => {
                file.read_to_end(&mut buffer).await?;
                println!("{}", buffer.len());
            }

            Err(e) => {
                println!("Error opening file: {}", e);
                return Err(Box::new(e));
            }
        }

        Ok(buffer)
    }

    async fn check_offset_range_in_file(
        offset_file_path: &str,
        offset_range: &Range<u64>,
    ) -> io::Result<bool> {
        let mut file = File::open(offset_file_path).await?;

        let starting_offset = offset_range.start;
        let ending_offset = offset_range.end;

        let mut buffer = Vec::new();
        file.read_to_end(&mut buffer).await?;

        let ranges = buffer
            .chunks_exact(8) // Assuming each value is 8 bytes (u64 size)
            .map(|chunk| {
                u64::from_le_bytes([
                    chunk[0], chunk[1], chunk[2], chunk[3], chunk[4], chunk[5], chunk[6], chunk[7],
                ])
            })
            .collect::<Vec<u64>>();

        for range in &ranges {
            if *range >= starting_offset && *range <= ending_offset {
                return Ok(true);
            }
        }

        Ok(false)
    }

    pub async fn write(
        &self,
        bytes: &[u8],
        archive_name: &str,
    ) -> Result<Range<u64>, Box<dyn Error>> {
        Self::create_dir(archive_name).await?;

        let offset_file_path = format!("{}/{}", archive_name, "offset");

        let offset_range = Self::write_to_offset_file(offset_file_path, bytes.len()).await?;

        let current_timestamp: DateTime<Utc> = Utc::now();
        let creation_date = current_timestamp.format("%Y-%m-%d %H:%M:%S UTC");

        let mut options = OpenOptions::new();
        options.write(true).create_new(true).read(true);

        let offset_range_string = format!("{}-{}", offset_range.start, offset_range.end);
        let bytes_file_path = format!("{}/{}", archive_name, offset_range_string);
        let manifest_file_path = format!("{}/{}.{}", archive_name, offset_range_string, "manifest");

        let content = Metadata {
            creation_date: creation_date.to_string(),
            range: offset_range_string,
            compression: "Gzip".to_string(),
        };
        let metadata = serde_json::to_string(&content).unwrap();

        Self::create_file(bytes_file_path, bytes).await?;
        Self::create_file(manifest_file_path, metadata.as_bytes()).await?;

        Ok(offset_range)
    }

    pub async fn read(&self, file_name: &str) -> Result<Vec<u8>, Box<dyn Error>> {
        let parts: Vec<&str> = file_name.splitn(2, '#').collect();
        let archive_name = parts[0];
        let offset_range = parts[1];
        let bytes_file_path = format!("{}/{}", archive_name, offset_range);

        let path = Path::new(&bytes_file_path);
        let mut bytes = Vec::new();

        if path.exists() {
            bytes = Self::read_file(bytes_file_path).await?;
        } else {
            return Err(Box::new(io::Error::new(
                io::ErrorKind::Other,
                "Error: File does not exist in EFS",
            )));
        }

        Ok(bytes)
    }

    pub async fn get_directories_list(&self, directory_path: &str) -> io::Result<Vec<String>> {
        let mut directories = Vec::new();

        let mut dir = fs::read_dir(directory_path).await?;

        while let Some(directory) = dir.next_entry().await? {
            let path = directory.path();
            if path.is_dir() {
                if let Some(directory_name) = path.file_name() {
                    if let Some(directory_name_str) = directory_name.to_str() {
                        directories.push(directory_name_str.to_string());
                    }
                }
            }
        }

        Ok(directories)
    }

    pub async fn delete(&self, archive_name: &str) -> Result<(), Box<dyn Error>> {
        fs::remove_dir_all(archive_name).await?;
        Ok(())
    }
}

#[cfg(test)]
mod efs_facade_test {
    use super::*;
    use std::{fs::File, io::Read};

    #[tokio::test]
    async fn test_read() {
        // Prepare
        let archive_name = "archive-test";
        let data = "Lorem Ipsum is simply dummy text of the printing and typesetting industry. Lorem Ipsum has been the industry's standard dummy text ever since the 1500s, when an unknown printer took a galley of type and scrambled it to make a type specimen book. It has survived not only five centuries, but also the leap into electronic typesetting, remaining essentially unchanged. It was popularised in the 1960s with the release of Letraset sheets containing Lorem Ipsum passages, and more recently with desktop publishing software like Aldus PageMaker including versions of Lorem Ipsum.";
        let bytes = data.as_bytes();

        let efs_facade = EfsFacade::new();
        let offset1 = efs_facade.write(&bytes, archive_name).await;
        let offset2 = efs_facade.write(&bytes, archive_name).await;

        let file_name = "archive-test#0-574";

        let result = efs_facade.read(file_name).await;
        assert_eq!(bytes, result.unwrap());

        let completed = efs_facade.delete(archive_name).await;
        assert!(completed.is_ok());
    }

    #[tokio::test]
    async fn test_write() {
        // Prepare
        let file_name = "archive-test";
        let data = "Lorem Ipsum is simply dummy text of the printing and typesetting industry. Lorem Ipsum has been the industry's standard dummy text ever since the 1500s, when an unknown printer took a galley of type and scrambled it to make a type specimen book. It has survived not only five centuries, but also the leap into electronic typesetting, remaining essentially unchanged. It was popularised in the 1960s with the release of Letraset sheets containing Lorem Ipsum passages, and more recently with desktop publishing software like Aldus PageMaker including versions of Lorem Ipsum.";
        let bytes = data.as_bytes();

        let efs_facade = EfsFacade::new();

        // Act
        let result = efs_facade.write(&bytes, file_name).await;

        // Assert
        match result {
            Ok(offset_range) => {
                let offset_range_string = format!("{}-{}", offset_range.start, offset_range.end);

                let bytes_file_path = format!("{}/{}", file_name, offset_range_string);
                let mut bytes_file =
                    File::open(bytes_file_path).expect("Failed to open the bytes file");

                let offset_file_path = format!("{}/{}", file_name, "offset");
                let mut offset_file =
                    File::open(offset_file_path).expect("Failed to open the offset file");

                let manifest_file_path =
                    format!("{}/{}.{}", file_name, offset_range_string, "manifest");

                let mainfest_path = Path::new(&manifest_file_path);

                let mut buffer = Vec::new();
                bytes_file
                    .read_to_end(&mut buffer)
                    .expect("Failed to read the bytes file");

                let actual_bytes = String::from_utf8_lossy(&buffer);

                let mut buffer = Vec::new();
                offset_file
                    .read_to_end(&mut buffer)
                    .expect("Failed to read the offset file");

                assert_eq!(actual_bytes, data);
                assert_eq!(offset_file.metadata().unwrap().len(), 8);
                assert!(mainfest_path.exists());

                let completed = efs_facade.delete(file_name).await;
                assert!(completed.is_ok());
            }
            Err(err) => {
                panic!("Error occurred during the test: {:?}", err);
            }
        }
    }
}
