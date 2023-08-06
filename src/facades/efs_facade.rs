use std::process::id;
use chrono::{Utc, Datelike};
use tokio::{fs::OpenOptions, io::{AsyncWriteExt, AsyncSeekExt, AsyncReadExt, self}};


fn get_current_date() -> String {
    let current_date = Utc::now();
    format!("{:04}-{:02}-{:02}", current_date.year(), current_date.month(), current_date.day())
}

fn get_file_path(collection: String) -> String {
    format!("{collection}-{pid}-{date}", collection=collection, pid = id(), date = get_current_date())
}


pub async fn append_bytes_collection(collection: String, bytes: Vec<u8>) -> Result<(String, u64, u64), String> {
    let file_path = get_file_path(collection);

    //We append to a file. If file doesn't exists, we create it.
    match OpenOptions::new().append(true).create(true).open(&format!("{}.gzip", file_path)).await {
        Ok(mut file) => {
            let before_size = file.metadata().await.map_err(|e| e.to_string())?.len();
            file.write_all(bytes.as_slice()).await.map_err(|e| e.to_string())?;
            let after_size = file.metadata().await.map_err(|e| e.to_string())?.len();
            Ok((file_path, before_size, after_size))
        },
        Err(error) => match error.kind() {
            _ => Err(error.to_string()),
        },
    }
}

pub async fn get_collection_byte_range(file_path: String, start: u64, end: u64) -> Result<Option<Vec<u8>>, String> {
    match OpenOptions::new().read(true).open(&format!("{}.gzip", file_path)).await {
        Ok(mut file) => {
            let mut buffer: Vec<u8> = Vec::new();
            file.seek(tokio::io::SeekFrom::Start(start)).await.map_err(|e| e.to_string())?;
            let mut chunk = file.take(end - start);
            chunk.read_to_end(&mut buffer).await.map_err(|e| e.to_string())?;
            Ok(Some(buffer))
        },
        Err(err) => match err.kind() {
            io::ErrorKind::NotFound => Ok(None),
            _ => Err(err.to_string()),
        }
    }
}