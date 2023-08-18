use chrono::{Datelike, Utc};
use serde::{Deserialize, Serialize};
use std::{alloc::System, env, process::id};
use tokio::{
    fs::{self, OpenOptions},
    io::{self, AsyncReadExt, AsyncSeekExt, AsyncWriteExt},
};

use super::compression;

fn get_current_date() -> String {
    let current_date = Utc::now();
    format!(
        "{:04}-{:02}-{:02}",
        current_date.year(),
        current_date.month(),
        current_date.day()
    )
}

pub fn get_file_path(collection: String) -> String {
    format!(
        "{collection}-{pid}-{date}",
        collection = collection,
        pid = id(),
        date = get_current_date()
    )
}

pub async fn get_directories_list(directory_path: &str) -> Result<Vec<String>, String> {
    let mut directories = Vec::new();

    let mut dir = fs::read_dir(directory_path)
        .await
        .map_err(|err| err.to_string())?;

    while let Ok(Some(directory)) = dir.next_entry().await {
        let path = directory.path();
        if path.is_dir() {
            if let Some(directory_name) = path.file_name() {
                if let Some(directory_name_str) = directory_name.to_str() {
                    directories.push(directory_name_str.to_string());
                }
            }
        }
    }

    Ok(directories)
}

pub async fn append_bytes_collection(
    collection: String,
    bytes: Vec<u8>,
) -> Result<(String, u64, u64), String> {
    let file_path = get_file_path(collection);
    let base = env::var("BASE_PATH").unwrap_or('/'.to_string()).to_string();
    //We append to a file. If file doesn't exists, we create it.
    match OpenOptions::new()
        .append(true)
        .create(true)
        .open(&format!(
            "{base}/{file_path}.gzip",
            base = base,
            file_path = file_path
        ))
        .await
    {
        Ok(mut file) => {
            file.write_all(bytes.as_slice())
                .await
                .map_err(|e| e.to_string())?;
            let after_size = file.seek(io::SeekFrom::Current(0)).await.unwrap();
            let before_size = after_size - bytes.len() as u64;
            //println!("BEFORE => {}\nAFTER=> {}", before_size, after_size);
            Ok((file_path, before_size, after_size))
        }
        Err(error) => match error.kind() {
            _ => Err(error.to_string()),
        },
    }
}

pub async fn get_collection_byte_range(
    file_path: String,
    start: u64,
    end: u64,
) -> Result<Option<Vec<u8>>, String> {
    let base = env::var("BASE_PATH").unwrap_or('/'.to_string()).to_string();
    match OpenOptions::new()
        .read(true)
        .open(&format!("{base}/{}.gzip", file_path, base = base))
        .await
    {
        Ok(mut file) => {
            let mut buffer: Vec<u8> = Vec::new();
            file.seek(tokio::io::SeekFrom::Start(start))
                .await
                .map_err(|e| e.to_string())?;
            let mut chunk = file.take(end - start);
            chunk
                .read_to_end(&mut buffer)
                .await
                .map_err(|e| e.to_string())?;
            Ok(Some(buffer))
        }
        Err(err) => match err.kind() {
            io::ErrorKind::NotFound => Ok(None),
            _ => Err(err.to_string()),
        },
    }
}

#[derive(Debug, Serialize, Deserialize)]
pub struct Metadata {
    creation_date: String,
    content_type: String,
    compression: String,
    source: String,
    start: u64,
    end: u64,
}

impl Metadata {
    pub fn new(
        content_type: String,
        compression: String,
        source: String,
        start: u64,
        end: u64,
    ) -> Metadata {
        Metadata {
            creation_date: Utc::now().format("%Y-%m-%d %H:%M:%S UTC").to_string(),
            content_type: content_type,
            compression: compression,
            source: source,
            start: start,
            end: end,
        }
    }
}

pub async fn write_metadata(collection: String, meta: Metadata) -> Result<(), String> {
    let file_path = get_file_path(collection);
    let base = env::var("BASE_PATH").unwrap_or('/'.to_string()).to_string();
    let meta_str = format!(
        "{}\n",
        serde_json::to_string(&meta).map_err(|e| e.to_string())?
    );

    //We append to a file. If file doesn't exists, we create it.
    match OpenOptions::new()
        .append(true)
        .create(true)
        .open(&format!(
            "{base}/{file_path}.manifest",
            base = base,
            file_path = file_path
        ))
        .await
    {
        Ok(mut file) => {
            file.write_all(meta_str.as_bytes())
                .await
                .map_err(|e| e.to_string())?;
            Ok(())
        }
        Err(e) => Err(e.to_string()),
    }
}
