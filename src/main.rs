pub mod middlewares;
use middlewares::tracing::tracing_fn;

pub mod handlers;
use handlers::collections::collection_handler;
use handlers::general::pong;

pub mod facades;
use facades::postgres_facade::{create_config_from_env, create_pool};

use crate::middlewares::tracing;
use axum::{
    middleware,
    routing::{any, get},
    Router,
};
use std::{env, net::SocketAddr};

#[derive(Clone)]
pub struct Config {
    pub secret: String,
}

fn create_addr(host: &str, port: &str) -> Result<SocketAddr, String> {
    let format = format!("{}:{}", host, port);
    format
        .parse::<SocketAddr>()
        .map_err(|_| format!("{} is not a valid app address", format))
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    tracing::init_tracing_with_jaeger()?;

    let postgres_config = create_config_from_env().expect("Unable to load");
    let postgres_pool = create_pool(postgres_config, 3).unwrap();

    // build our application with a route
    let app = Router::new()
        .route("/ping", get(pong))
        .route("/collection/*collection", any(collection_handler))
        .with_state(postgres_pool)
        .layer(middleware::from_fn(tracing_fn));

    let app_host = env::var("APP_HOST").unwrap_or("0.0.0.0".to_string());
    let app_port = env::var("APP_PORT").unwrap_or("5000".to_string());
    //Create app url
    let addr = create_addr(&app_host, &app_port);

    match addr {
        Ok(valid_addr) => {
            // run it
            println!("listening on {}", valid_addr);
            axum::Server::bind(&valid_addr)
                .serve(app.into_make_service())
                .await
                .unwrap();
        }
        Err(err) => println!("ABORTING => {}", err.to_string()),
    }
    Ok(())
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
