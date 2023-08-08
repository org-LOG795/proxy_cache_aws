pub mod middlewares;
use middlewares::tracing::tracing_fn;

pub mod handlers;
use handlers::general::pong;
use handlers::collections::collection_handler;
use handlers::metrics::handle_metrics;

pub mod facades;

use axum::{
    routing::{any, get},
    Router,
    middleware
};
use std::{env, net::SocketAddr};
use crate::middlewares::tracing;
use crate::facades::compression::{gzip_compress, gzip_decompress};


#[derive(Clone)]
pub struct Config {
    pub secret: String,
}

fn create_addr(host: &str, port: &str) -> Result<SocketAddr, String> {
    let format = format!("{}:{}", host, port);
    format.parse::<SocketAddr>().map_err(|_| format!("{} is not a valid app address", format))
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    tracing::init_tracing()?;

    // build our application with a route
    let app = Router::new()
        .route("/ping", get(pong))
        .route("/collection/*collection", any(collection_handler))
        .route("/metrics", get(handle_metrics))
        .layer(middleware::from_fn(tracing_fn));

    let app_host = env::var("APP_HOST").unwrap_or("0.0.0.0".to_string());
    let app_port = env::var("APP_PORT").unwrap_or("5000".to_string());
    //Create app url
    let addr = create_addr(&app_host, &app_port);
    
    test_prometheus();

    match addr {
        Ok(valid_addr) => {
            // run it
            println!("listening on {}", valid_addr);
            axum::Server::bind(&valid_addr)
                .serve(app.into_make_service())
                .await
                .unwrap();
        }
        Err(err) => println!("ABORTING => {}", err.to_string())
    }
    
    Ok(())
}

fn test_prometheus() {
    // Données à compresser
    let data = b"Hello, world!";
    println!("Original data: {:?}", data);

    // Compresser les données
    let compressed_data = gzip_compress(data.to_vec()).unwrap();
    println!("Compressed data: {:?}", compressed_data);

    // Décompresser les données
    let decompressed_data = gzip_decompress(compressed_data).unwrap();
    println!("Decompressed data: {:?}", decompressed_data);
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn create_valid_addr() {
        let addr = create_addr("127.0.0.1", "5000");
        assert!(addr.is_ok())
    }
    #[test]
    fn create_invalid_addr() {
        let addr = create_addr("123.456.789", "ab99");
        assert!(addr.is_err())
    }
}
