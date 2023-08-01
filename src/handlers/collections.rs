use std::collections::HashMap;

use super::super::facades;
use axum::extract::{Path, Query, State};
use axum::response::IntoResponse;
use deadpool_postgres::{Object, Pool};
use facades::compression::gzip_compress;
use facades::efs_facade::{read as read_efs, write as write_efs};
use facades::postgres_facade::get_offset;
use facades::s3::{init_client as init_s3_client, read_file as read_s3};
use hyper::body::to_bytes;
use hyper::{Body, Method, Request, StatusCode};

pub async fn collection_handler(
    Path(collection): Path<String>,
    State(pool_manager): State<Pool>,
    request: Request<Body>,
) -> impl IntoResponse {
    match *request.method() {
        Method::GET => {
            let params = extract_query_params(&request.uri().to_string());
            match (
                params.get("start").map(|s| s.parse::<i64>()),
                params.get("end").map(|e| e.parse::<i64>()),
            ) {
                (Some(Ok(start)), Some(Ok(end))) => {
                    match get_handler(collection, start, end).await {
                        Ok(Some(bytes)) => (StatusCode::OK, bytes).into_response(),
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
            let client = pool_manager.get().await.unwrap();
            let bytes = to_bytes(request.into_body()).await.unwrap().to_vec();

            match post_handler(collection, bytes, client).await {
                Ok(file_path) => (StatusCode::OK, file_path).into_response(),
                Err(err) => (StatusCode::INTERNAL_SERVER_ERROR, err).into_response(),
            }
        }
        Method::PATCH => {
            let params = extract_query_params(&request.uri().to_string());
            let bytes = to_bytes(request.into_body()).await.unwrap().to_vec();
            match (
                params.get("start").map(|s| s.parse::<i64>()),
                params.get("end").map(|e| e.parse::<i64>()),
            ) {
                (Some(Ok(start)), Some(Ok(end))) => {
                    match patch_handler(collection, bytes, start, end).await {
                        Ok(file_path) => (StatusCode::OK, file_path).into_response(),
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
        _ => StatusCode::NOT_FOUND.into_response(),
    }
}

/*Steps
1. extract archive and range from reference
2. Check efs (return if found)
3. Check S3 (return if found)
4. If nothing found... cry :(
*/
async fn get_handler(collection: String, start: i64, end: i64) -> Result<Option<Vec<u8>>, String> {
    let mut res: Option<Vec<u8>> = read_efs(&collection, start, end).await?;

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
    postgres_client: Object,
) -> Result<String, String> {
    //Compress
    match gzip_compress(bytes) {
        Ok(compressed) => {
            if let Ok(offsets) = get_offset(postgres_client, compressed.len()).await {
                write_efs(compressed, &collection, offsets.0, offsets.1)
                    .await
                    .map(|_| {
                        format!(
                            "{collection}?start={start}&end={end}",
                            collection = collection,
                            start = offsets.0,
                            end = offsets.1
                        )
                        .to_string()
                    })
                    .map_err(|err| err.to_string())
            } else {
                Err("Unable to get offset".to_string())
            }
        }
        Err(_) => Err("Unable to compress".to_string()),
    }
}
async fn patch_handler(
    collection: String,
    bytes: Vec<u8>,
    start: i64,
    end: i64,
) -> Result<String, String> {
    //Compress
    match gzip_compress(bytes) {
        Ok(compressed) => write_efs(compressed, &collection, start, end)
            .await
            .map(|_| {
                format!(
                    "{collection}?start={start}&end={end}",
                    collection = collection,
                    start = start,
                    end = end
                )
                .to_string()
            })
            .map_err(|err| err.to_string()),
        Err(_) => Err("Unable to compress".to_string()),
    }
}

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
    use facades::postgres_facade::{create_config, create_pool};
    use tokio_postgres::Config;

    fn get_test_config() -> Config {
        create_config("127.0.0.1", "guest", "guest", "test")
    }

    async fn reset_offset_table(client: Object) -> Result<(), ()> {
        let query = "DELETE FROM public.\"CacheOffsetTable\" WHERE true;";
        let statement = client.prepare_cached(&query).await;

        assert!(statement.is_ok());

        client.query(&statement.unwrap(), &[]).await.unwrap();
        Ok(())
    }

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

    #[tokio::test]
    #[ignore = "postgres"]
    async fn save_test() {
        let pg_config = get_test_config();
        let pool = create_pool(pg_config, 1);
        assert!(pool.is_ok());
        let client = pool.unwrap().get().await.unwrap();

        let test_base = "UN_FICHIER_TRES_PETIT".to_string();
        let path = Path::new("test_files/poem.txt");
        let bytes: Vec<u8> = fs::read(path).unwrap();

        assert!(post_handler(test_base, bytes, client).await.is_ok())
    }

    #[tokio::test]
    #[ignore = "postgres"]
    async fn get_test() {
        let pg_config = get_test_config();
        let pool = create_pool(pg_config, 1);
        assert!(pool.is_ok());
        let client = pool.unwrap().get().await.unwrap();

        let test_base = "UN_FICHIER_TRES_PETIT".to_string();
        let path = Path::new("test_files/poem.txt");
        let bytes: Vec<u8> = fs::read(path).unwrap();

        assert!(post_handler(test_base, bytes, client).await.is_ok())
    }

    #[tokio::test]
    #[ignore = "postgres"]
    async fn get_post_integration_test() {
        let pg_config = get_test_config();
        let pool = create_pool(pg_config, 1).unwrap();

        let client = pool.get().await.unwrap();
        //Make sure we are ready for the test
        assert!(reset_offset_table(client).await.is_ok());

        let test_files = load_test_files();

        let test_collection_name = "test_collection".to_string();

        for bytes in test_files.iter() {
            let client = pool.get().await.unwrap();
            let collection_name = test_collection_name.clone();
            //Add the value
            let post_res = post_handler(collection_name, bytes.to_vec(), client).await;
            assert!(post_res.is_ok());

            let file_path = post_res.unwrap();

            let params = extract_query_params(&file_path);

            let start = params.get("start").unwrap().parse::<i64>().unwrap();
            let end = params.get("end").unwrap().parse::<i64>().unwrap();
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
