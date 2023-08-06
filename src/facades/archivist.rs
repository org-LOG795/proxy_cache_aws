use serde::{Deserialize, Serialize};
use std::error::Error;
use std::path::Path;
use tokio::fs::{self, File, OpenOptions};
use tokio::io::{self, AsyncBufReadExt, AsyncReadExt, AsyncWriteExt};
use tracing_subscriber::filter::Directive;

use super::efs_facade::{self, Metadata};
use super::s3::{self};

#[derive(Debug, Serialize, Deserialize)]
struct Manifest {
    name: String,
    meta_source: String,
    segments: Vec<Metadata>,
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

        let mut files_list = match fs::read_dir(directory_path).await {
            Ok(files) => files,
            Err(err) => {
                return Err(format!("Error reading directory: {}", err));
            }
        };

        let mut options = OpenOptions::new();
        let output_options = options.write(true).append(true).create(true);

        let mut combined_manifest = Vec::<Metadata>::new();
        let mut output_file_name = format!("{}", directory);

        while let Ok(Some(file)) = files_list.next_entry().await {
            let path = file.path();

            if let Some(file_name) = path.file_name().and_then(|os_str| os_str.to_str()) {
                let file_path = format!("{}/{}/{}", master_directory_path, directory, file_name);

                if file_name.contains(".manifest") {
                    let manifest_segment = read_from_manifest(&file_path).await;
                    combined_manifest.extend(manifest_segment.unwrap());
                } else {
                    let bytes = efs_facade::read_file(&file_path.to_string()).await;

                    let output_file_path = format!(
                        "{}/{}/{}",
                        master_directory_path, directory, output_file_name
                    );

                    let _ = efs_facade::write_file(
                        &output_file_path,
                        &bytes.unwrap().as_slice(),
                        output_options,
                    )
                    .await;
                }
            }
        }

        let manifest = Manifest {
            name: output_file_name,
            meta_source: "src".to_string(),
            segments: combined_manifest,
        };
        let json_manifest = serde_json::to_string_pretty(&manifest);

        let manifest_file_name = format!("{}-manifest.json", directory);
        let manifest_file_path = format!(
            "{}/{}/{}",
            master_directory_path, directory, manifest_file_name
        );

        let _ = efs_facade::write_file(
            &manifest_file_path,
            json_manifest.unwrap().as_bytes(),
            output_options,
        )
        .await;

        // s3::upload_file_multipart(bucket_name, &directory_path, &directory, part_size).await;
        let directory_path_for_delete = format!("{}/{}", master_directory_path, directory);
        efs_facade::delete(&directory_path_for_delete).await;
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
        let test_file_path = format!("{}", "test_files/lorem.txt");

        let data = efs_facade::read_file(&test_file_path).await;
        assert!(data.is_ok());

        let bytes = data.unwrap();

        efs_facade::write(bytes.clone(), file_name, 0, 574).await;
        efs_facade::write(bytes.clone(), file_name, 574, 1148).await;
        efs_facade::write(bytes.clone(), file_name_2, 0, 574).await;

        let archivist = archive_to_s3(directory_name, "bucket", 64).await;
        assert!(archivist.is_ok());
        let deleted = efs_facade::delete(directory_name).await;
        assert!(deleted.is_ok());
    }
}
