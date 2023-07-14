use std::path::Path;
use tokio::fs;
use std::io;

pub struct EfsFacade {
}


impl<'a> EfsFacade<'a> {
    pub fn new() -> Self {

    }

    pub async fn write(&self, bytes: &[u8], archive_name: String) -> Result<(), String> {

        file_path = ""

        let mut options = OpenOptions::new();
        options.write(true).create_new(true);

        match options.open(file_path).await {
            Ok(mut file) => {

                let current_offset = file.seek(SeekFrom::Current(0)).await.unwrap();

                if let Err(err) = file.write_all(bytes).await {
                    println!("Error writing to offset file: {}", err);
                }

                println!("Data written to the offset file");
                
                let new_offset = file.seek(SeekFrom::Current(0)).await.unwrap();
                let offset_range = current_offset..new_offset;
            }
            Err(err) => {
                println!("Error opening file: {}", err);
            }
        }

        return offset_range
    }

    pub async fn read(&mut self, file_name: &String) -> Result<(), u8> {
        
        // Need to determine the file naming convention
        let parts: Vec<&str> = file_name.split('_').collect();

        if let Some(offset_range) = parts.get(1) {
            
            let offsets: Vec<&str> = offset_range.split('-').collect();

            if let (Some(start), Some(end)) = (offsets.get(0), offsets.get(1)) {
                
                if let Ok(start_offset) = start.parse::<u64>() {
                    if let Ok(end_offset) = end.parse::<u64>() {
                        
                        let mut options = OpenOptions::new();
                        let mut file = options.read(true).open(file_name).await.unwrap();
                            
                        file.seek(SeekFrom::Start(start_offset)).await.unwrap();
                        
                        let bytes_range = (end_offset - start_offset) as usize;

                        
                        let mut bytes = vec![0u8; bytes_range];
                        file.read_exact(&mut bytes).await.unwrap();

                        
                        return bytes;
                    }
                }
            }
        }

        Err("Error occured whiel reading bytes".into())
        
    }
}