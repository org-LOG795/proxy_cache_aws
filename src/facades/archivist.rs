use std::error::Error;
use std::path::Path;
use tokio::fs::{self, File, OpenOptions};
use tokio::io::{self, AsyncReadExt, AsyncWriteExt};
use tracing_subscriber::filter::Directive;

use super::efs_facade::{self};
use super::s3::{self};

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

        let mut files_list = match fs::read_dir(directory_path).await {
            Ok(files) => files,
            Err(err) => {
                return Err(format!("Error reading directory: {}", err));
            }
        };

        let mut options = OpenOptions::new();
        let output_options = options.write(true).append(true).create(true);

        while let Ok(Some(file)) = files_list.next_entry().await {
            let path = file.path();
            let mut output_file_name = format!("{}", directory);

            if let Some(file_name) = path.file_name().and_then(|os_str| os_str.to_str()) {
                if file_name.contains(".manifest") {
                    output_file_name = format!("{}.json", directory);
                }

                println!("{}", file_name);
                let file_path = format!("{}/{}/{}", master_directory_path, directory, file_name);

                let bytes = efs_facade::read_file(file_path.to_string()).await;

                let output_file_path = format!(
                    "{}/{}/{}",
                    master_directory_path, directory, output_file_name
                );

                let _ = efs_facade::create_file(
                    &output_file_path,
                    &bytes.unwrap().as_slice(),
                    output_options,
                )
                .await;
            }
            // s3::upload_file_multipart(bucket_name, &directory_path, &directory, part_size).await;
            // efs_facade::delete(&directory_path).await;
        }
    }

    Ok(())
}

async fn get_file_size(file_path: &str) -> u64 {
    if let Ok(metadata) = fs::metadata(file_path).await {
        metadata.len()
    } else {
        0
    }
}

fn calculate_part_size(file_size: u64) -> usize {
    // Adjust the part size calculation according to the file size
    if file_size > 5_000_000_000_000 {
        // If file size is larger than 5TB
        100_000_000 // Use a part size of 100MB
    } else if file_size > 100_000_000 {
        // If file size is larger than 100MB
        10_000_000 // Use a part size of 10MB
    } else {
        // If file size is smaller than 100MB
        8_000_000 // Use the default part size of 8MB
    }
}

#[cfg(test)]
mod archivist_test {
    use super::*;
    use tokio::fs;

    use crate::facades::efs_facade;

    #[tokio::test]
    async fn test_archive_to_s3() {
        let directory_name = "test-directory";
        fs::create_dir(directory_name).await;
        let file_name = "test-directory/archive-test-write";
        let file_name_2 = "test-directory/archive-test-write-2";
        let data = "Lorem Ipsum is simply dummy text of the printing and typesetting industry. Lorem Ipsum has been the industry's standard dummy text ever since the 1500s, when an unknown printer took a galley of type and scrambled it to make a type specimen book. It has survived not only five centuries, but also the leap into electronic typesetting, remaining essentially unchanged. It was popularised in the 1960s with the release of Letraset sheets containing Lorem Ipsum passages, and more recently with desktop publishing software like Aldus PageMaker including versions of Lorem Ipsum.";
        let bytes = data.as_bytes();

        efs_facade::write((&bytes).to_vec(), file_name, 0, 574).await;
        efs_facade::write((&bytes).to_vec(), file_name, 574, 1148).await;
        efs_facade::write((&bytes).to_vec(), file_name_2, 0, 574).await;

        let archivist = archive_to_s3(directory_name, "bucket", 64).await;
        assert!(archivist.is_ok());
    }
}
