use chrono::prelude::*;
use serde::{Deserialize, Serialize};
use std::error::Error;
use std::ops::Range;
use std::path::Path;
use tokio::fs::{self, OpenOptions};
use tokio::io::{AsyncReadExt, AsyncWriteExt};

#[derive(Debug, Serialize, Deserialize)]
pub struct Metadata {
    start: i64,
    end: i64,
    compression: String,
    creation_date: String,
}

async fn create_dir(dir_name: &str) -> Result<(), tokio::io::Error> {
    let path = Path::new(dir_name);

    match fs::create_dir(path).await {
        Ok(_) => {
            println!("Created directory: {:#?}", path);
            Ok(())
        }
        Err(err) if err.kind() == tokio::io::ErrorKind::AlreadyExists => {
            println!("Directory already exists: {:#?}", path);
            Ok(())
        }
        Err(err) => Err(err),
    }
}

pub async fn write_file(
    file_path: &String,
    bytes: &[u8],
    options: &mut OpenOptions,
) -> Result<(), Box<dyn Error>> {
    match options.open(file_path).await {
        Ok(mut file) => {
            file.write_all(bytes).await?;
            println!("Bytes were written into file: {}", file_path);
        }

        Err(e) => {
            println!("Error writting into file: {}, {}", file_path, e);
            return Err(Box::new(e));
        }
    }

    Ok(())
}

pub async fn read_file(file_path: &String) -> Result<Vec<u8>, Box<dyn Error>> {
    let mut options = OpenOptions::new();
    options.read(true);
    let mut buffer = Vec::new();

    match options.open(file_path).await {
        Ok(mut file) => {
            file.read_to_end(&mut buffer).await?;
            println!("Bytes were read from file: {}", file_path);
        }

        Err(e) => {
            println!("Error reading file: {}, {}", file_path, e);
            return Err(Box::new(e));
        }
    }

    Ok(buffer)
}

pub async fn write(bytes: Vec<u8>, archive_name: &str, start: i64, end: i64) -> Result<(), String> {
    create_dir(archive_name).await;

    let creation_date = Utc::now().format("%Y-%m-%d %H:%M:%S UTC");

    let offset_range_string = format!("{}-{}", start, end);
    let bytes_file_path = format!("{}/{}", archive_name, offset_range_string);
    let manifest_file_path = format!("{}/{}.{}", archive_name, offset_range_string, "manifest");

    let content = Metadata {
        start: start,
        end: end,
        compression: "GZ".to_string(),
        creation_date: creation_date.to_string(),
    };
    let metadata = serde_json::to_string(&content).unwrap();

    let mut options = OpenOptions::new();
    let output_options = options.write(true).create_new(true);

    write_file(&bytes_file_path, bytes.as_slice(), output_options).await;
    write_file(&manifest_file_path, metadata.as_bytes(), output_options).await;

    Ok(())
}

pub async fn read(archive_name: &str, start: i64, end: i64) -> Result<Option<Vec<u8>>, String> {
    let bytes_file_path = format!("{}/{}", archive_name, format!("{}-{}", start, end));
    let path = Path::new(&bytes_file_path);

    if path.exists() {
        let bytes = read_file(&bytes_file_path).await;

        bytes.map(|b| Some(b)).map_err(|err| err.to_string())
    } else {
        Ok(None)
    }
}

pub async fn get_directories_list(directory_path: &str) -> Result<Vec<String>, String> {
    let mut directories = Vec::new();

    let mut dir = fs::read_dir(directory_path)
        .await
        .map_err(|err| err.to_string())?;

    while let Ok(Some(directory)) = dir.next_entry().await {
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

pub async fn delete(archive_name: &str) -> Result<(), Box<dyn Error>> {
    fs::remove_dir_all(archive_name).await?;
    println!("File path deleted: {}", archive_name);
    Ok(())
}

#[cfg(test)]
mod efs_facade_test {
    use super::*;
    use std::{fs::File, io::Read};

    #[tokio::test]
    async fn test_read() {
        let archive_name = "archive-test-read";
        let test_file_path = format!("{}", "test/lorem.txt");

        let data = read_file(&test_file_path).await;
        assert!(data.is_ok());

        let bytes = data.unwrap();

        let offset = write(bytes.clone(), archive_name, 0, 574).await;
        assert!(offset.is_ok());

        let result = read(archive_name, 0, 574).await;
        assert_eq!(result, Ok(Some(bytes)));

        let completed = delete(archive_name).await;
        assert!(completed.is_ok());
    }

    #[tokio::test]
    async fn test_write() {
        // Prepare
        let file_name = "archive-test-write";
        let test_file_path = format!("{}", "test/lorem.txt");

        let data = read_file(&test_file_path).await;
        assert!(data.is_ok());

        let bytes = data.unwrap();

        // Act
        let result = write(bytes.clone(), file_name, 0, 574).await;

        // Assert
        match result {
            Ok(()) => {
                let offset_range_string = format!("{}-{}", 0, 574);

                let bytes_file_path = format!("{}/{}", file_name, offset_range_string);
                let mut bytes_file =
                    File::open(bytes_file_path).expect("Failed to open the bytes file");

                let manifest_file_path =
                    format!("{}/{}.{}", file_name, offset_range_string, "manifest");

                let mainfest_path = Path::new(&manifest_file_path);

                let mut buffer = Vec::new();
                bytes_file
                    .read_to_end(&mut buffer)
                    .expect("Failed to read the bytes file");

                let actual_bytes = String::from_utf8_lossy(&buffer);
                let original_bytes = String::from_utf8_lossy(&bytes);

                assert_eq!(actual_bytes, original_bytes);
                assert!(mainfest_path.exists());

                let completed = delete(file_name).await;
                assert!(completed.is_ok());
            }
            Err(err) => {
                panic!("Error occurred during the test: {:?}", err);
            }
        }
    }

    #[tokio::test]
    #[ignore = "The order of result gets messed up in Github"]
    async fn test_get_directories_list() {
        create_dir("test-dir").await;
        create_dir("test-dir/test1").await;
        create_dir("test-dir/test2").await;
        create_dir("test-dir/test3").await;

        let result = get_directories_list("test-dir").await;

        let actual: Vec<String> = vec![
            "test1".to_string(),
            "test2".to_string(),
            "test3".to_string(),
        ];

        assert_eq!(result, Ok(actual));

        let completed = delete("test-dir").await;
        assert!(completed.is_ok());
    }
}
