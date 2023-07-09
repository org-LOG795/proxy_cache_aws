use rusoto_core::Region;
use rusoto_s3::{ListBucketsOutput, PutObjectRequest, S3Client, S3};
use std::error::Error;
use std::fs::File;
use std::io::prelude::*;
use tokio::time::{sleep, Duration};

pub struct S3Facade {
    client: S3Client,
}

// Minimum part size for S3 is 5MB
// Maximmim nuber of parts is 10000
// Current part size allows for 5 * 10000 = 50GB size
const PART_SIZE: usize = 5_242_880;

// Maximum attempts for file upload
const MAX_UPLOAD_ATTEMPTS: u32 = 5;

impl S3Facade {
    pub fn new() -> Self {
        let region = Region::default();
        let client = S3Client::new(region);
        S3Facade { client }
    }

    // Upload file to specific bucket
    pub async fn upload_file(
        &self,
        bucket_name: &str,
        file_path: &str,
        file_name: &str,
    ) -> Result<(), Box<dyn Error>> {
        let mut file = File::open(file_path)?;
        let mut buffer = Vec::new();
        file.read_to_end(&mut buffer)?;

        let put_object_req = PutObjectRequest {
            bucket: bucket_name.to_owned(),
            key: file_name.to_owned(),
            body: Some(buffer.into()),
            ..Default::default()
        };
        self.client.put_object(put_object_req).await?;

        Ok(())
    }

    // Upload multipart file to specific bucket
    pub async fn upload_file_multipart(
        &self,
        bucket_name: &str,
        file_path: &str,
        file_name: &str,
        part_size: usize,
    ) -> Result<(), Box<dyn Error>> {
        let mut file = File::open(file_path)?;
        let mut buffer = vec![0; part_size];
        let mut part_number = 1;
        let mut completed_parts = Vec::new();

        let create_req = rusoto_s3::CreateMultipartUploadRequest {
            bucket: bucket_name.to_owned(),
            key: file_name.to_owned(),
            ..Default::default()
        };
        let upload_output = self.client.create_multipart_upload(create_req).await?;
        let upload_id = upload_output.upload_id.ok_or("Missing upload ID")?;

        while let Ok(n) = file.read(&mut buffer) {
            if n == 0 {
                break;
            }

            let mut attempts = 0;

            loop {
                let part_req = rusoto_s3::UploadPartRequest {
                    bucket: bucket_name.to_owned(),
                    key: file_name.to_owned(),
                    upload_id: upload_id.clone(),
                    part_number,
                    body: Some(buffer[..n].to_vec().into()),
                    ..Default::default()
                };

                // Upload part with retries based on set value
                match self.client.upload_part(part_req).await {
                    Ok(part_output) => {
                        completed_parts.push(rusoto_s3::CompletedPart {
                            e_tag: part_output.e_tag.clone(),
                            part_number: Some(part_number),
                        });
                        println!(
                            "Uploaded part {} with ETag {}",
                            part_number,
                            part_output.e_tag.clone().unwrap_or_default()
                        );
                        part_number += 1;
                        break;
                    }
                    Err(e) => {
                        attempts += 1;
                        if attempts >= MAX_UPLOAD_ATTEMPTS {
                            return Err(Box::new(e));
                        } else {
                            // Exponential backoff: Wait for 2^(attempts - 1) seconds
                            sleep(Duration::from_secs(2u64.pow(attempts - 1))).await;
                        }
                    }
                }
            }
        }

        let complete_req = rusoto_s3::CompleteMultipartUploadRequest {
            bucket: bucket_name.to_owned(),
            key: file_name.to_owned(),
            upload_id,
            multipart_upload: Some(rusoto_s3::CompletedMultipartUpload {
                parts: Some(completed_parts),
            }),
            ..Default::default()
        };

        let complete_output = self.client.complete_multipart_upload(complete_req).await?;
        let file_etag = complete_output.e_tag.ok_or("Missing file ETag")?;

        println!("File ETag: {}", file_etag);
        println!("File Key: {}", file_name);

        Ok(())
    }

    // list S3 buckets
    pub async fn list_buckets(&self) -> Result<Vec<String>, Box<dyn Error>> {
        let response: ListBucketsOutput = self.client.list_buckets().await?;
        let bucket_names = response
            .buckets
            .unwrap_or_default()
            .into_iter()
            .map(|bucket| bucket.name.unwrap_or_default())
            .collect();

        Ok(bucket_names)
    }

    // create s3 bucket
    pub async fn create_bucket(&self, bucket_name: &str) -> Result<(), Box<dyn Error>> {
        let create_bucket_req = rusoto_s3::CreateBucketRequest {
            bucket: bucket_name.to_owned(),
            ..Default::default()
        };
        self.client.create_bucket(create_bucket_req).await?;

        Ok(())
    }

    pub async fn delete_bucket(&self, bucket_name: &str) -> Result<(), Box<dyn Error>> {
        let delete_bucket_req = rusoto_s3::DeleteBucketRequest {
            bucket: bucket_name.to_owned(),
            expected_bucket_owner: None,
        };
        self.client.delete_bucket(delete_bucket_req).await?;

        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use rusoto_core;
    use std::io::Write;
    use tempfile::NamedTempFile;
    use tokio::time::{sleep, Duration};
    use uuid::Uuid;

    // Minimum part size for S3 is 5MB
    // Maximmim nuber of parts is 10000
    // Current part size allows for 5 * 10000 = 50GB size
    const PART_SIZE: usize = 5_242_880;

    #[tokio::test]
    async fn test_list_buckets() {
        let s3_facade = S3Facade::new();
        let id = Uuid::new_v4();
        let bucket_name = format!("test-bucket-{}", id);

        // Create a bucket
        let create_result = s3_facade.create_bucket(&bucket_name).await;
        assert!(create_result.is_ok());

        // List buckets
        let list_result = s3_facade.list_buckets().await;
        if let Err(e) = list_result {
            println!("Error: {:?}", e);
            panic!();
        }

        // Print buckets
        let buckets = list_result.unwrap();
        println!("Bucket names:");
        for bucket in &buckets {
            println!("{}", bucket);
        }

        // Check if the new bucket is in the list
        assert!(buckets.contains(&bucket_name));

        // Cleanup
        let delete_result = s3_facade.delete_bucket(&bucket_name).await;
        assert!(delete_result.is_ok());
    }

    #[tokio::test]
    async fn test_create_bucket() {
        let s3_facade = S3Facade::new();
        let id = Uuid::new_v4();
        let bucket_name = format!("test-bucket-{}", id);

        // Delete bucket if exists
        let _ = s3_facade.delete_bucket(&bucket_name).await;
        let result = s3_facade.create_bucket(&bucket_name).await;

        if let Err(e) = result {
            println!("Error: {:?}", e);
            panic!();
        }

        assert!(result.is_ok());
    }

    #[tokio::test]
    async fn test_upload_file() {
        let s3_facade = S3Facade::new();
        let id = Uuid::new_v4();
        let bucket_name = format!("test-bucket-{}", id);

        // Create a new bucket for the test
        let create_result = s3_facade.create_bucket(&bucket_name).await;
        assert!(create_result.is_ok());

        // Create temp file
        let mut temp_file = NamedTempFile::new().unwrap();
        write!(temp_file, "This is a test file").unwrap();
        let file_name = "file.txt";
        let file_path = temp_file.path().to_str().unwrap();

        // Upload the file
        let upload_result = s3_facade
            .upload_file(&bucket_name, file_path, file_name)
            .await;
        if let Err(e) = upload_result {
            println!("Error: {:?}", e);
            panic!();
        }

        assert!(upload_result.is_ok());

        // Cleanup
        let delete_object_req = rusoto_s3::DeleteObjectRequest {
            bucket: bucket_name.clone(),
            key: file_name.to_owned(),
            ..Default::default()
        };
        let _ = s3_facade
            .client
            .delete_object(delete_object_req)
            .await
            .unwrap();

        let delete_result = s3_facade.delete_bucket(&bucket_name).await;
        if let Err(e) = delete_result {
            println!("Error: {:?}", e);
            panic!();
        }

        assert!(delete_result.is_ok());
    }

    #[tokio::test]
    async fn test_upload_file_multipart() {
        let s3_facade = S3Facade::new();
        let id = Uuid::new_v4();
        let bucket_name = format!("test-bucket-{}", id);

        // Create a new bucket for the test
        let create_result = s3_facade.create_bucket(&bucket_name).await;
        assert!(create_result.is_ok(), "Bucket creation failed");

        // File path
        let file_name = "";
        let file_path = "";

        // Upload file
        let upload_result = s3_facade
            .upload_file_multipart(&bucket_name, file_path, file_name, PART_SIZE)
            .await;
        if let Err(e) = upload_result {
            if let Some(rusoto_err) =
                e.downcast_ref::<rusoto_core::RusotoError<rusoto_s3::UploadPartError>>()
            {
                match rusoto_err {
                    rusoto_core::RusotoError::HttpDispatch(dispatch_error) => {
                        println!("This was an HttpDispatch error: {:?}", dispatch_error);
                    }
                    rusoto_core::RusotoError::Service(service_error) => {
                        println!("This was a Service error: {:?}", service_error);
                    }
                    rusoto_core::RusotoError::Unknown(unknown_error) => {
                        println!("This was an Unknown error: {:?}", unknown_error);
                    }
                    //Other error handling here
                    _ => {}
                }
            }
            panic!("File upload failed");
        }
        assert!(upload_result.is_ok(), "File upload assertion failed");

        // Cleanup
        let delete_object_req = rusoto_s3::DeleteObjectRequest {
            bucket: bucket_name.clone(),
            key: file_name.to_owned(),
            ..Default::default()
        };
        let delete_object_result = s3_facade.client.delete_object(delete_object_req).await;
        assert!(delete_object_result.is_ok(), "Object deletion failed");

        let delete_bucket_result = s3_facade.delete_bucket(&bucket_name).await;
        if let Err(e) = delete_bucket_result {
            println!("Error: {:?}", e);
            panic!("Bucket deletion failed");
        }

        assert!(
            delete_bucket_result.is_ok(),
            "Bucket deletion assertion failed!"
        );
    }
}
