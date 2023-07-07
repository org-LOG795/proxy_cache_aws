use rusoto_core::Region;
use rusoto_s3::{ListBucketsOutput, PutObjectRequest, S3, S3Client};
use std::error::Error;
use std::fs::File;
use std::io::prelude::*;

pub struct S3Facade {
    client: S3Client,
}

impl S3Facade {
    pub fn new() -> Self {
        let region = Region::default();
        let client = S3Client::new(region);
        S3Facade { client }
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

    // Upload file to an existing S3 bucket
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
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::io::Write;
    use tempfile::NamedTempFile;
    use uuid::Uuid;


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
        
        // Verify that the new bucket is in the list
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

}
