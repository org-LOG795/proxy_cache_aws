use std::error::Error;

use tracing_subscriber::filter::Directive;

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
    pub async fn archive_to_s3(&self, file_name: &str, bucket_name: &str, part_size: usize) -> Result<(), Box<dyn Error>> {
        let path = "temp";
        let directories_list = self.efs_facade.get_directories_list(path);
        let file_bytes = self.efs_facade.read(file_name).await?;
        self.s3_facade.upload_file_multipart(bucket_name, file_name, file_name, part_size).await?;
        Ok(())
    }

    async fn get_files_from_efs(&self){
        let path = "temp";
        let directories_list = self.efs_facade.get_directories_list(path).await;

        for directories in &directories_list{

            // if manifest file: add into a json

            // if file: send to S3 bucket


        }
    }

}

