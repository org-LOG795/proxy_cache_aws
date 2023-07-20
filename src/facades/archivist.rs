use std::error::Error;
use crate::efs_facade::EfsFacade;
use crate::s3_facade::S3Facade;

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
    pub async fn archive(&self, file_name: &str, bucket_name: &str, part_size: usize) -> Result<(), Box<dyn Error>> {
        let file_bytes = self.efs_facade.read(file_name).await?;
        self.s3_facade.upload_file_multipart(bucket_name, file_name, file_name, part_size).await?;
        Ok(())
    }

}
