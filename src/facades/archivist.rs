use serde::{Deserialize, Serialize};
use serde_json::to_string;
use std::error::Error;
use std::path::Path;
use tokio::fs::{self, File, OpenOptions};
use tokio::io::{self, AsyncBufReadExt, AsyncReadExt, AsyncWriteExt, Error as IoError};
use tracing_subscriber::filter::Directive;

use super::efs_facade::{self, Metadata};
use super::s3::{self};

async fn read_file(file_path: &String) -> Result<Vec<u8>, Box<dyn Error>> {
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

async fn write_file(file_path: &String, bytes: &[u8]) -> Result<(), Box<dyn Error>> {
    let mut options = OpenOptions::new();
    let output_options = options.write(true).create_new(true);

    match output_options.open(file_path).await {
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

//Read from EFS and write to an S3 bucket
pub async fn archive_to_s3(
    master_directory_path: &str,
    bucket_name: &str,
    part_size: usize,
) -> Result<(), String> {
    let directories_list = efs_facade::get_directories_list(master_directory_path).await;

    let directories_list = match directories_list {
        Ok(directories_list) => directories_list,
        Err(err) => {
            return Err(format!("Error fetching directories: {}", err));
        }
    };

    for directory in directories_list {
        let directory_path = format!("{}/{}", master_directory_path, directory);
        let file_size = get_file_size(&directory_path.clone()).await;
        let part_size = calculate_part_size(file_size);

        let s3_client = s3::init_client();

        let output_file_name = format!("{}", directory);
        let output_file_path = format!("{}/{}", directory_path, directory);

        if let Ok(_metadata) = fs::metadata(output_file_path).await {
            match s3::upload_file_multipart(
                bucket_name,
                &directory_path.clone(),
                &output_file_name,
                part_size,
                s3_client.clone(),
            )
            .await
            {
                Ok(_) => (),
                Err(e) => return Err(format!("Failed to upload bytes file: {}", e)),
            }
        } else {
            return Err(format!("Bytes file does not exist"));
        }

        let manifest_file_name = format!("{}.manifest", directory);
        let manifest_file_path = format!("{}/{}", directory_path, manifest_file_name);

        if let Ok(_metadata) = fs::metadata(manifest_file_path.clone()).await {
            let manifest_bytes = read_file(&manifest_file_path).await;

            let json_manifest_name = format!("{}-manifest.json", directory);
            let json_manifest_path = format!("{}/{}", directory_path, json_manifest_name);

            let _ = write_file(&json_manifest_path, &manifest_bytes.unwrap()).await;
            match s3::upload_file_multipart(
                bucket_name,
                &directory_path.clone(),
                &json_manifest_name,
                part_size,
                s3_client.clone(),
            )
            .await
            {
                Ok(_) => (),
                Err(e) => return Err(format!("Failed to upload manifest file: {}", e)),
            }
        } else {
            return Err(format!("Manifest file does not exist"));
        }

        let directory_path_for_delete = format!("{}/{}", master_directory_path, directory);
        let _ = fs::remove_dir_all(directory_path_for_delete).await;
        println!("File path deleted: {}", directory);
    }

    Ok(())
}

async fn read_from_manifest(file_path: &str) -> Result<Vec<Metadata>, Box<dyn std::error::Error>> {
    let path = Path::new(file_path);
    let file = File::open(path).await?;
    let buf_reader = tokio::io::BufReader::new(file);
    let mut segments = Vec::new();

    let mut lines = buf_reader.lines();
    while let Some(line) = lines.next_line().await? {
        let metadata: Metadata = serde_json::from_str(&line)?;
        segments.push(metadata);
    }

    Ok(segments)
}

async fn get_file_size(file_path: &str) -> u64 {
    if let Ok(metadata) = fs::metadata(file_path).await {
        metadata.len()
    } else {
        0
    }
}

fn calculate_part_size(file_size: u64) -> usize {
    if file_size > 5_000_000_000_000 {
        100_000_000 //100MB
    } else if file_size > 100_000_000 {
        10_000_000 //10MB
    } else {
        8_000_000 //8MB
    }
}

#[cfg(test)]
mod archivist_test {
    use super::*;
    use tokio::fs;

    #[tokio::test]
    async fn test_archive_to_s3() {
        let directory_name = "test-directory";

        //let archivist = archive_to_s3(directory_name, "bucket", 64).await;
        let archivist = archive_to_s3(directory_name, "rusty-bucket-2834", 1).await;
        assert!(archivist.is_ok());
        // let deleted = efs_facade::delete(directory_name).await;
        // assert!(deleted.is_ok());
    }
}
