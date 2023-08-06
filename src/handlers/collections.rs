use std::collections::HashMap;

use super::super::facades;
use axum::extract::{State, Path, Query};
use axum::{response::IntoResponse, http::{StatusCode, header::{self, HeaderMap, HeaderName}}};
use deadpool_postgres::{Object, Pool};
use facades::compression::gzip_compress;
use facades::efs_facade::{get_collection_byte_range as read_efs, append_bytes_collection as write_efs};
use facades::s3::{read_file as read_s3, init_client as init_s3_client};
use hyper::body::to_bytes;
use hyper::{Request, Body, Method};

pub async fn collection_handler(
    Path(collection): Path<String>,
    request: Request<Body>
) -> impl IntoResponse {
        match *request.method() {
            Method::GET => {
                let params = extract_query_params(&request.uri().to_string());
                match (params.get("start").map(|s| s.parse::<u64>()), params.get("end").map(|e| e.parse::<u64>())) {
                    (Some(Ok(start)), Some(Ok(end))) => {
                        match get_handler(collection, start, end).await {
                            Ok(Some(bytes)) => {
                                let mut headers = HeaderMap::new();
                                headers.insert(header::CONTENT_TYPE, "application/octet-stream".parse().unwrap());
                                headers.insert(header::CONTENT_ENCODING, "gzip".parse().unwrap());
                                (StatusCode::OK, headers, bytes).into_response()
                            },
                            Ok(None) => (StatusCode::NOT_FOUND, "unable to find supplied byte range".to_string()).into_response(),
                            Err(err) => (StatusCode::INTERNAL_SERVER_ERROR, err).into_response(),
                        }
                    },
                    _ => (StatusCode::BAD_REQUEST, "unable to parse start & end params".to_string()).into_response()
                }
            },
            Method::POST => {
                let bytes = to_bytes(request.into_body()).await.unwrap().to_vec();

                match post_handler(collection, bytes).await{
                    Ok(file_path) => (StatusCode::OK, file_path).into_response(),
                    Err(err) => (StatusCode::INTERNAL_SERVER_ERROR, err).into_response()
                }
            },
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
            _ => StatusCode::NOT_FOUND.into_response()
        }
}

/*Steps
1. extract archive and range from reference
2. Check efs (return if found)
3. Check S3 (return if found)
4. If nothing found... cry :(
*/
async fn get_handler(collection: String, start: u64, end: u64) -> Result<Option<Vec<u8>>, String>{
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
async fn post_handler(collection: String, bytes: Vec<u8>) -> Result<String, String> {
    match gzip_compress(bytes) {
        Ok(compressed) => {
            write_efs(collection, compressed).await
            .map(|t|format!("{file}?start={start}&end={end}",file = t.0, start = t.1, end = t.2).to_string())
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
    use std::{path::Path, fs};

    use chrono::{Utc, Datelike};
    use tokio_postgres::Config;
    use super::*;
    use facades::compression::{gzip_decompress as decompress, gzip_compress as compress};

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
        let path_str = format!("test/collections_testing/test_files/test_{}.txt", index).to_string();
        let path = Path::new(&path_str);

        //unwrap because it should be ok
        fs::read(path).unwrap()
    }

    #[tokio::test]
    async fn get_post_integration_test() {
        let test_files = load_test_files();

        let test_collection_name = "test_collection".to_string();

        for bytes in test_files.iter() {
            let collection_name = test_collection_name.clone();
            //Add the value
            let post_res = post_handler(collection_name, bytes.to_vec()).await;
            assert!(post_res.is_ok());

            let file_path = post_res.unwrap();

            let params = extract_query_params(&file_path);

            let start = params.get("start").unwrap().parse::<u64>().unwrap();
            let end = params.get("end").unwrap().parse::<u64>().unwrap();
            let get_res = get_handler(test_collection_name.clone(), start, end).await;
            assert!(get_res.is_ok());
            assert!(get_res.as_ref().unwrap().is_some());

            let compressed_bytes = get_res.unwrap().unwrap();

            //Validate that the get function works
            assert_eq!(compress(bytes.clone()).unwrap(), compressed_bytes);

            //Make sure the values are the same
            assert_eq!(decompress(compressed_bytes.clone()).unwrap(), bytes.clone())
        }

        //Cleanup
        delete_test_folders()
    }

}