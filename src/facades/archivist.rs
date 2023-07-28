use std::error::Error;
use tokio::fs::{self, File};
use tracing_subscriber::filter::Directive;
use tokio::io::AsyncWriteExt;

use super::{efs_facade::EfsFacade, s3::S3Facade};

pub struct Archivist {
    efs_facade: EfsFacade,
    s3_facade: S3Facade,
}

impl Archivist {
   
    pub fn new(efs_facade: EfsFacade, s3_facade: S3Facade) -> Self {
        Archivist {
            efs_facade,
            s3_facade,
        }
    }

    //Read from EFS and write to an S3 bucket
    pub async fn archive_to_s3(
        &self,
        file_name: &str,
        bucket_name: &str,
        part_size: usize,
    ) -> Result<(), Box<dyn Error>> {
        let path = "temp";

        //TODO : name combined file accordigly
        let combined_file_name = "combined_file.txt"; 
        self.get_files_from_efs().await?; // Combine the files and write to combined_file.txt

        // Upload the combined file to S3 using multipart
        self.s3_facade
            .upload_file_multipart(bucket_name, &combined_file_name, file_name, part_size)
            .await?;

        // Optional: Delete the combined file after uploading to S3
        let combined_file_path = format!("{}/{}", path, combined_file_name);
        fs::remove_file(&combined_file_path).await?;

        Ok(())
    }

    async fn get_files_from_efs(&self) -> Result<(), Box<dyn Error>> {
        let path = "temp";
        let directories_list = self.efs_facade.get_directories_list(path).await?;

        let mut combined_bytes = Vec::new();

        for directory_name in &directories_list {

            let manifest_str = ".manifest".to_string();

            // Skip manifest files and offset
            if !directory_name.ends_with(manifest_str.as_str()) && *directory_name != "offset" {
                let file_path = format!("{}/{}", path, directory_name);
                let file_bytes = self.efs_facade.read(&file_path).await?;
                combined_bytes.extend_from_slice(&file_bytes);
            }
        }

        // Write the combined bytes to temp file
        let combined_file_name = "combined_file.txt";
        let combined_file_path = format!("{}/{}", path, combined_file_name);
        let mut combined_file = File::create(&combined_file_path).await?;
        combined_file.write_all(&combined_bytes).await?; 

        Ok(())

    }


    async fn get_file_size(&self, file_path: &str) -> u64 {
        if let Ok(metadata) = fs::metadata(file_path).await {
            metadata.len()
        } else {
            0
        }
    }

    fn calculate_part_size(&self, file_size: u64) -> usize {
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

}


#[cfg(test)]
mod tests {

}
