use dotenv::dotenv;
use log::info;
use rusoto_core::Region;
use rusoto_s3::{ListBucketsOutput, PutObjectRequest, S3Client, S3};
use std::{error::Error, ops::Range};
use std::fs::File;
use std::io::prelude::*;
use tokio::time::{sleep, Duration};

pub fn init_client() -> S3Client {
    dotenv().ok();

    let region = Region::default();
    S3Client::new(region)
}

// Minimum part size for S3 is 5MB
// Maximmim nuber of parts is 10000
// Current part size allows for 50 * 10000 = 50GB size

// Maximum attempts for file and multipart uploads
const MAX_UPLOAD_ATTEMPTS: u32 = 5;

// Upload file to specific bucket
pub async fn upload_file(
    bucket_name: &str,
    file_path: &str,
    file_name: &str,
    client: S3Client
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
    client.put_object(put_object_req).await?;

    Ok(())
}

pub async fn read_file(file_name: &str, client: S3Client, start: i64, end: i64) -> Result<Vec<u8>, String> {
    //read_bytes
    //let bytes = S3.read()

    //get bytes range
    //let wanted_bytes = bytes.range(range)

    //return
    //Ok(wanted_bytes)
    todo!()
} 

// Upload multipart file to specific bucket
pub async fn upload_file_multipart(
    bucket_name: &str,
    file_path: &str,
    file_name: &str,
    part_size: usize,
    client: S3Client
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
    let upload_output = client.create_multipart_upload(create_req).await?;
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
            match client.upload_part(part_req).await {
                Ok(part_output) => {
                    completed_parts.push(rusoto_s3::CompletedPart {
                        e_tag: part_output.e_tag.clone(),
                        part_number: Some(part_number),
                    });
                    info!(
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
                        // Abort upload
                        let abort_req = rusoto_s3::AbortMultipartUploadRequest {
                            bucket: bucket_name.to_owned(),
                            key: file_name.to_owned(),
                            upload_id: upload_id.clone(),
                            ..Default::default()
                        };
                        client.abort_multipart_upload(abort_req).await?;
                        info!("Upload of file aborted: {}", file_name);
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

    let complete_output = client.complete_multipart_upload(complete_req).await?;
    let file_etag = complete_output.e_tag.ok_or("Missing file ETag")?;

    info!("Uploaded file ETag: {}", file_etag);
    info!("Uploaded file Key: {}", file_name);

    Ok(())
}

// Abort multipart upload
pub async fn abort_multipart_upload(
    bucket_name: &str,
    key: &str,
    upload_id: &str,
    client: S3Client
) -> Result<(), Box<dyn Error>> {
    let abort_req = rusoto_s3::AbortMultipartUploadRequest {
        bucket: bucket_name.to_owned(),
        key: key.to_owned(),
        upload_id: upload_id.to_owned(),
        ..Default::default()
    };
    client.abort_multipart_upload(abort_req).await?;

    Ok(())
}

// list S3 buckets
pub async fn list_buckets(client: S3Client) -> Result<Vec<String>, Box<dyn Error>> {
    let response: ListBucketsOutput = client.list_buckets().await?;
    let bucket_names = response
        .buckets
        .unwrap_or_default()
        .into_iter()
        .map(|bucket| bucket.name.unwrap_or_default())
        .collect();

    Ok(bucket_names)
}

// create s3 bucket
pub async fn create_bucket(bucket_name: &str, client: S3Client) -> Result<(), Box<dyn Error>> {
    let create_bucket_req = rusoto_s3::CreateBucketRequest {
        bucket: bucket_name.to_owned(),
        ..Default::default()
    };
    client.create_bucket(create_bucket_req).await?;

    Ok(())
}

pub async fn delete_bucket(bucket_name: &str, client: S3Client) -> Result<(), Box<dyn Error>> {
    let delete_bucket_req = rusoto_s3::DeleteBucketRequest {
        bucket: bucket_name.to_owned(),
        expected_bucket_owner: None,
    };
    client.delete_bucket(delete_bucket_req).await?;

    Ok(())
}

// #[cfg(test)]
// mod tests {
//     use super::*;
//     use rusoto_core;
//     use std::io::Write;
//     use tempfile::NamedTempFile;
//     use uuid::Uuid;

//     // Minimum part size for S3 is 5MB
//     // Maximmim nuber of parts is 10000
//     // Current part size allows for 50 * 10000 = 50GB size
//     const PART_SIZE: usize = 5_242_8800;

//     #[tokio::test]
//     #[ignore = "tests need to be ran with AWS credentials defined in environment"]
//     async fn test_list_buckets() {
//         let s3_facade = S3Facade::new();
//         let id = Uuid::new_v4();
//         let bucket_name = format!("test-bucket-{}", id);

//         // Create a bucket
//         let create_result = s3_facade.create_bucket(&bucket_name).await;
//         assert!(create_result.is_ok());

//         // List buckets
//         let list_result = s3_facade.list_buckets().await;
//         if let Err(e) = list_result {
//             info!("Error: {:?}", e);
//             panic!();
//         }

//         // Print buckets
//         let buckets = list_result.unwrap();
//         println!("Bucket names:");
//         for bucket in &buckets {
//             info!("{}", bucket);
//         }

//         // Check if the new bucket is in the list
//         assert!(buckets.contains(&bucket_name));

//         // Cleanup
//         let delete_result = s3_facade.delete_bucket(&bucket_name).await;
//         assert!(delete_result.is_ok());
//     }

//     #[tokio::test]
//     #[ignore = "tests need to be ran with AWS credentials defined in environment"]
//     async fn test_create_bucket() {
//         let s3_facade = S3Facade::new();
//         let id = Uuid::new_v4();
//         let bucket_name = format!("test-bucket-{}", id);

//         // Delete bucket if exists
//         let _ = s3_facade.delete_bucket(&bucket_name).await;
//         let result = s3_facade.create_bucket(&bucket_name).await;

//         if let Err(e) = result {
//             println!("Error: {:?}", e);
//             panic!();
//         }

//         assert!(result.is_ok());
//     }

//     #[tokio::test]
//     #[ignore = "tests need to be ran with AWS credentials defined in environment"]
//     async fn test_upload_file() {
//         let s3_facade = S3Facade::new();
//         let id = Uuid::new_v4();
//         let bucket_name = format!("test-bucket-{}", id);

//         // Create a new bucket for the test
//         let create_result = s3_facade.create_bucket(&bucket_name).await;
//         assert!(create_result.is_ok());

//         // Create temp file
//         let mut temp_file = NamedTempFile::new().unwrap();
//         write!(temp_file, "This is a test file").unwrap();
//         let file_name = "file.txt";
//         let file_path = temp_file.path().to_str().unwrap();

//         // Upload the file
//         let upload_result = s3_facade
//             .upload_file(&bucket_name, file_path, file_name)
//             .await;
//         if let Err(e) = upload_result {
//             info!("Error: {:?}", e);
//             panic!();
//         }

//         assert!(upload_result.is_ok());

//         // Cleanup
//         let delete_object_req = rusoto_s3::DeleteObjectRequest {
//             bucket: bucket_name.clone(),
//             key: file_name.to_owned(),
//             ..Default::default()
//         };
//         let _ = s3_facade
//             .client
//             .delete_object(delete_object_req)
//             .await
//             .unwrap();

//         let delete_result = s3_facade.delete_bucket(&bucket_name).await;
//         if let Err(e) = delete_result {
//             info!("Error: {:?}", e);
//             panic!();
//         }

//         assert!(delete_result.is_ok());
//     }

//     #[tokio::test]
//     #[ignore = "tests need to be ran with AWS credentials defined in environment"]
//     async fn test_upload_file_multipart() {
//         let s3_facade = S3Facade::new();
//         let id = Uuid::new_v4();
//         let bucket_name = format!("test-bucket-{}", id);

//         // Create a new bucket for the test
//         let create_result = s3_facade.create_bucket(&bucket_name).await;
//         assert!(create_result.is_ok(), "Bucket creation failed");

//         // Generate a file for upload
//         let mut temp_file = tempfile::NamedTempFile::new().unwrap();
//         let file_data = vec![0; PART_SIZE + 10]; // Some data larger than one part
//         temp_file.write_all(&file_data).unwrap();
//         let file_path = temp_file.path().to_str().unwrap().to_owned();
//         let file_name = "multipart_upload.txt";

//         // Upload file
//         let upload_result = s3_facade
//             .upload_file_multipart(&bucket_name, &file_path, file_name, PART_SIZE)
//             .await;
//         if let Err(e) = upload_result {
//             if let Some(rusoto_err) =
//                 e.downcast_ref::<rusoto_core::RusotoError<rusoto_s3::UploadPartError>>()
//             {
//                 match rusoto_err {
//                     rusoto_core::RusotoError::HttpDispatch(dispatch_error) => {
//                         info!("This was an HttpDispatch error: {:?}", dispatch_error);
//                     }
//                     rusoto_core::RusotoError::Service(service_error) => {
//                         info!("This was a Service error: {:?}", service_error);
//                     }
//                     rusoto_core::RusotoError::Unknown(unknown_error) => {
//                         info!("This was an Unknown error: {:?}", unknown_error);
//                     }
//                     //Other error handling hereA
//                     _ => {}
//                 }
//             }
//             panic!("File upload failed");
//         }
//         assert!(upload_result.is_ok(), "File upload assertion failed");

//         // Cleanup
//         let delete_object_req = rusoto_s3::DeleteObjectRequest {
//             bucket: bucket_name.clone(),
//             key: file_name.to_owned(),
//             ..Default::default()
//         };
//         let delete_object_result = s3_facade.client.delete_object(delete_object_req).await;
//         assert!(delete_object_result.is_ok(), "Object deletion failed");

//         let delete_bucket_result = s3_facade.delete_bucket(&bucket_name).await;
//         if let Err(e) = delete_bucket_result {
//             info!("Error: {:?}", e);
//             panic!("Bucket deletion failed");
//         }

//         assert!(
//             delete_bucket_result.is_ok(),
//             "Bucket deletion assertion failed!"
//         );
//     }

//     #[tokio::test]
//     #[ignore = "tests need to be ran with AWS credentials defined in environment"]
//     async fn test_abort_multipart_upload() {
//         let s3_facade = S3Facade::new();
//         let id = Uuid::new_v4();
//         let bucket_name = format!("test-bucket-{}", id);

//         // Create a new bucket for the test
//         let create_result = s3_facade.create_bucket(&bucket_name).await;
//         assert!(create_result.is_ok());

//         // File path
//         let file_name = "file.txt";

//         // Start multipart upload
//         let create_req = rusoto_s3::CreateMultipartUploadRequest {
//             bucket: bucket_name.clone(),
//             key: file_name.to_owned(),
//             ..Default::default()
//         };
//         let upload_output = s3_facade
//             .client
//             .create_multipart_upload(create_req)
//             .await
//             .unwrap();
//         let upload_id = upload_output.upload_id.unwrap();

//         // Force an error by using an invalid upload_id
//         let invalid_upload_id = "invalid_id";
//         let part_req = rusoto_s3::UploadPartRequest {
//             bucket: bucket_name.clone(),
//             key: file_name.to_owned(),
//             upload_id: invalid_upload_id.to_string(),
//             part_number: 1,
//             body: None, // No actual part
//             ..Default::default()
//         };

//         let upload_result = s3_facade.client.upload_part(part_req).await;

//         // Assert that the upload failed
//         assert!(upload_result.is_err());

//         // Abort
//         let abort_result = s3_facade
//             .abort_multipart_upload(&bucket_name, file_name, &upload_id)
//             .await;
//         assert!(abort_result.is_ok());

//         // Cleanup
//         let delete_result = s3_facade.delete_bucket(&bucket_name).await;

//         assert!(delete_result.is_ok());
//     }
// }
