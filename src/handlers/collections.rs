use std::collections::HashMap;
use std::time::Instant;

use crate::facades::efs_facade::Metadata;

use super::super::facades;
use axum::extract::{Path, Query, State};
use axum::{
    http::{
        header::{self, HeaderMap, HeaderName},
        StatusCode,
    },
    response::IntoResponse,
};
use deadpool_postgres::{Object, Pool};
use facades::compression::gzip_compress;
use facades::efs_facade::{
    append_bytes_collection as write_efs, get_collection_byte_range as read_efs, write_metadata,
};
use facades::s3::{init_client as init_s3_client, read_file as read_s3};
use hyper::body::to_bytes;
use hyper::{Body, Method, Request};

pub async fn collection_handler(
    Path(collection): Path<String>,
    request: Request<Body>,
) -> impl IntoResponse {
    match *request.method() {
        Method::GET => {
            let params = extract_query_params(&request.uri().to_string());
            match (
                params.get("start").map(|s| s.parse::<u64>()),
                params.get("end").map(|e| e.parse::<u64>()),
            ) {
                (Some(Ok(start)), Some(Ok(end))) => {
                    match get_handler(collection, start, end).await {
                        Ok(Some(bytes)) => {
                            let mut headers = HeaderMap::new();
                            headers.insert(
                                header::CONTENT_TYPE,
                                "application/octet-stream".parse().unwrap(),
                            );
                            headers.insert(header::CONTENT_ENCODING, "gzip".parse().unwrap());
                            (StatusCode::OK, headers, bytes).into_response()
                        }
                        Ok(None) => (
                            StatusCode::NOT_FOUND,
                            "unable to find supplied byte range".to_string(),
                        )
                            .into_response(),
                        Err(err) => (StatusCode::INTERNAL_SERVER_ERROR, err).into_response(),
                    }
                }
                _ => (
                    StatusCode::BAD_REQUEST,
                    "unable to parse start & end params".to_string(),
                )
                    .into_response(),
            }
        }
        Method::POST => {
            let content_type = request
                .headers()
                .get("Content-Type")
                .unwrap()
                .to_str()
                .unwrap()
                .to_string();
            let host = request
                .headers()
                .get("Host")
                .unwrap()
                .to_str()
                .unwrap()
                .to_string();
            let bytes = to_bytes(request.into_body()).await.unwrap().to_vec();
            match post_handler(collection, bytes, content_type, host).await {
                Ok(file_path) => (StatusCode::OK, file_path).into_response(),
                Err(err) => (StatusCode::INTERNAL_SERVER_ERROR, err).into_response(),
            }
        }
        // Method::PATCH => {
        //     let params = extract_query_params(&request.uri().to_string());
        //     let bytes = to_bytes(request.into_body()).await.unwrap().to_vec();
        //     match (params.get("start").map(|s| s.parse::<i64>()), params.get("end").map(|e| e.parse::<i64>())) {
        //         (Some(Ok(start)), Some(Ok(end))) => {
        //             match patch_handler(collection, bytes, start, end).await {
        //                 Ok(file_path) => (StatusCode::OK, file_path).into_response(),
        //                 Err(err) => (StatusCode::INTERNAL_SERVER_ERROR, err).into_response()
        //             }
        //         },
        //         _ => (StatusCode::BAD_REQUEST, "unable to parse start & end params".to_string()).into_response()
        //     }
        // },
        _ => StatusCode::NOT_FOUND.into_response(),
    }
}

/*Steps
1. extract archive and range from reference
2. Check efs (return if found)
3. Check S3 (return if found)
4. If nothing found... cry :(
*/
async fn get_handler(collection: String, start: u64, end: u64) -> Result<Option<Vec<u8>>, String> {
    let mut res: Option<Vec<u8>> = read_efs(collection, start, end).await?;

    if res.is_none() {
        //read S3
        // let client = init_s3_client();
        // res = read_s3(&collection, client, start, end).await.map(|val| Some(val))?;
    }

    Ok(res)
}

/*Steps
1. Compress bytes
2. Ask BD for current offset
3. Create file name
4. Send to EFS
5. Return file name
*/
async fn post_handler(
    collection: String,
    bytes: Vec<u8>,
    content_type: String,
    host: String,
) -> Result<String, String> {
    //Start the timer
    // let compress_start = Instant::now();
    match gzip_compress(bytes) {
        Ok(compressed) => {
            // println!("COMPRESS => {}ms", compress_start.elapsed().as_millis().to_string());

            // let write_efs_start = Instant::now();
            let write_res = write_efs(collection.clone(), compressed)
                .await
                .map_err(|e| e.to_string())?;
            let formatted_path = format!(
                "{file}?start={start}&end={end}",
                file = write_res.0,
                start = write_res.1,
                end = write_res.2
            )
            .to_string();
            // println!("EFS => {}ms", write_efs_start.elapsed().as_millis().to_string());

            // let meta_start = Instant::now();
            let meta = Metadata::new(
                content_type,
                "gzip".to_string(),
                host,
                write_res.1,
                write_res.2,
            );
            write_metadata(collection, meta).await?;
            // println!("META => {}ms", meta_start.elapsed().as_millis().to_string());

            Ok(formatted_path)
        }
        Err(_) => Err("Unable to compress".to_string()),
    }
}

// async fn patch_handler(collection: String, bytes: Vec<u8>, start: i64, end: i64) -> Result<String, String> {
//     //Compress
//     match gzip_compress(bytes) {
//         Ok(compressed) => {
//             write_efs(compressed, &collection, start, end).await
//                 .map(|_|format!("{collection}?start={start}&end={end}",collection = collection, start = start, end = end).to_string())
//                 .map_err(|err| err.to_string())
//         }
//         Err(_) => Err("Unable to compress".to_string()),
//     }
// }

fn extract_query_params(url: &str) -> HashMap<String, String> {
    let mut params: HashMap<String, String> = HashMap::new();

    if let Some(query_str) = url.split_once('?') {
        for pair in query_str.1.split('&') {
            if let Some((key, value)) = pair.split_once('=') {
                params.insert(key.to_string(), value.to_string());
            }
        }
    }

    params
}

#[cfg(test)]
mod tests {
    use std::{fs, path::Path};

    use super::*;
    use chrono::{Datelike, Utc};
    use facades::compression::{gzip_compress as compress, gzip_decompress as decompress};
    use facades::efs_facade::get_file_path;
    use tokio_postgres::Config;

    fn load_test_files() -> Vec<Vec<u8>> {
        let mut files: Vec<Vec<u8>> = Vec::new();

        for i in 1..=9 {
            files.push(load_test_file(i));
        }

        assert_eq!(files.len(), 9);

        files
    }

    fn delete_test_folders() {
        let dir_path = "test_collection";

        assert!(fs::remove_dir_all(dir_path).is_ok());
    }

    fn load_test_file(index: usize) -> Vec<u8> {
        let path_str =
            format!("test/collections_testing/test_files/test_{}.txt", index).to_string();
        let path = Path::new(&path_str);

        //unwrap because it should be ok
        fs::read(path).unwrap()
    }

    // #[tokio::test]
    // async fn get_post_integration_test() {
    //     let test_files = load_test_files();

    //     let test_collection_name = "test_collection".to_string();

    //     for bytes in test_files.iter() {
    //         let collection_name = test_collection_name.clone();
    //         //Add the value
    //         let post_res = post_handler(collection_name.clone(), bytes.to_vec()).await;
    //         assert!(post_res.is_ok());

    //         let file_path = post_res.unwrap();
    //         let params = extract_query_params(&file_path.clone());
    //         let start = params.get("start").unwrap().parse::<u64>().unwrap();
    //         let end = params.get("end").unwrap().parse::<u64>().unwrap();

    //         let get_res = get_handler(get_file_path(collection_name), start, end).await;
    //         assert!(get_res.is_ok());
    //         assert!(get_res.as_ref().unwrap().is_some());

    //         let compressed_bytes = get_res.unwrap().unwrap();

    //         //Validate that the get function works
    //         assert_eq!(compress(bytes.clone()).unwrap(), compressed_bytes);

    //         let decompress = decompress(compressed_bytes.clone()).unwrap();

    //         assert_eq!(decompress.len(), bytes.len());
    //         //Make sure the values are the same
    //         assert_eq!(decompress, bytes.clone())
    //     }
    // }
}
