use std::fs::OpenOptions;
use std::io::{Read, Seek, SeekFrom, Write};
use std::ops::RangeInclusive;
use std::path::Path;
use tokio::fs;

pub struct EfsFacade {}

impl<'a> EfsFacade<'a> {
    pub fn new() -> Self {
        EfsFacade {}
    }

    pub async fn write(
        &self,
        bytes: &[u8],
        archive_name: String,
    ) -> Result<(), RangeInclusive<u64>> {
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

    pub async fn read(
        &mut self,
        offset_range: &RangeInclusive<u64>,
        file_name: &str,
    ) -> Result<(), Vec<u8>> {
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

#[cfg(test)]
mod efs_facade_test {
    use super::*;
    use std::path::PathBuf;
    use tokio::io::AsyncWriteExt;

    #[tokio::test]
    async fn test_write() {
        // Prepare
        let temp_dir = tempfile::tempdir().expect("Failed to create a temporary directory");
        let file_name = temp_dir.path().join("test_file.txt");
        let data = "Hello, World!";
        let bytes = data.as_bytes();

        let efs_facade = EfsFacade::new();

        // Act
        let result = efs_facade.write(bytes, file_name.to_string()).await;

        // Assert
        assert!(result.is_ok(), "Failed to write bytes to the file");

        let mut file = tokio::fs::File::open(file_name)
            .await
            .expect("Failed to open the file");
        let mut buffer = Vec::new();
        file.read_to_end(&mut buffer)
            .await
            .expect("Failed to read the file");

        let actual = String::from_utf8_lossy(&buffer);

        assert_eq!(actual, data);
    }

    #[tokio::test]
    async fn test_read() {
        // Prepare
        let temp_dir = tempfile::tempdir().expect("Failed to create a temporary directory");
        let file_name = temp_dir.path().join("test_file.txt");
        let data = "Hello, World!";
        let bytes = data.as_bytes();

        let efs_facade = EfsFacade::new();

        // Act
        let write_result = efs_facade.write(bytes, file_name.to_string()).await;
        assert!(write_result.is_ok(), "Failed to write bytes to the file");

        let offset_range = 0..data.len() as u64;
        let result = efs_facade
            .read(&offset_range, file_name.to_str().unwrap())
            .await;

        // Assert
        assert!(result.is_ok(), "Failed to read bytes from the file");

        let actual = result.unwrap();
        assert_eq!(actual, data);
    }
}
