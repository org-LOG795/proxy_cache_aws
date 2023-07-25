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
                    // let current_offset: u64 = file.seek(SeekFrom::Current(0)).await?;
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

    async fn check_offset_range_in_file(
        offset_file_path: &str,
        offset_range: u8,
    ) -> io::Result<bool> {
        let mut file = File::open(offset_file_path).await?;

        let mut buffer = Vec::new();
        file.read_to_end(&mut buffer).await?;

        for byte in &buffer {
            if *byte == offset_range {
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

    // pub async fn read(
    //     &mut self,
    //     offset_range: &RangeInclusive<u64>,
    //     file_name: &str,
    // ) -> Result<(), Vec<u8>> {
    //     if let Some(start) = offset_range.start {
    //         if let Some(end) = offset_range.end {
    //             let mut options = OpenOptions::new();
    //             let mut file = options.read(true).open(file_name).await?;

    //             file.seek(SeekFrom::Start(start)).await?;

    //             let bytes_range = (end - start) as usize;
    //             let mut bytes = vec![0u8; bytes_range];
    //             file.read_exact(&mut bytes).await?;

    //             Ok(bytes);
    //         } else {
    //             Err("Error reading the end offset".into())
    //         }
    //     } else {
    //         Err("Error reading the start of the offset".into())
    //     }
    // }

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
    use std::{fs::File, io::Read};

    use super::*;

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

// #[tokio::test]
// async fn test_read() {
//     // Prepare
//     let temp_dir = tempfile::tempdir().expect("Failed to create a temporary directory");
//     let file_name = temp_dir.path().join("test_file.txt");
//     let data = "Hello, World!";
//     let bytes = data.as_bytes();

//     let efs_facade = EfsFacade::new();

//     // Act
//     let write_result = efs_facade.write(bytes, file_name.to_string()).await;
//     assert!(write_result.is_ok(), "Failed to write bytes to the file");

//     let offset_range = 0..data.len() as u64;
//     let result = efs_facade
//         .read(&offset_range, file_name.to_str().unwrap())
//         .await;

//     // Assert
//     assert!(result.is_ok(), "Failed to read bytes from the file");

//     let actual = result.unwrap();
//     assert_eq!(actual, data);
// }
// }
