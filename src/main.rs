pub mod middlewares;
pub mod facades;
use facades::s3::S3Facade;

use axum::{
    response::Html, 
    routing::get,
    Router,
    middleware,
    extract::State, Json,
};
use deadpool_postgres::{Pool};
use middlewares::tracing::tracing_fn;
use facades::postgres_facade::{create_config_from_env, create_pool};
use std::{env, net::SocketAddr};
use serde::{Serialize, Deserialize};

use crate::middlewares::tracing;


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
    tracing::init_tracing_with_jaeger()?;
    let secret_test = Config {secret: "olo".to_string()};

    let postgres_config = create_config_from_env().expect("Unable to load");
    let postgres_pool = create_pool(postgres_config, 3).unwrap();

    // build our application with a route
    let app = Router::new()
        .route("/", get(handler))
        .route("/secret", get(say_secret))
        .with_state(secret_test)
        .route("/postgres", get(postgres_test_handler))
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
        Err(err) => println!("ABORTING => {}", err.to_string())
    }
Ok(())
}

async fn handler() -> Html<&'static str> {
    Html("<h1>Hello, World!</h1>")
}

async fn say_secret(State(config) : State<Config>) -> String {
    return config.secret;
}

#[derive(Serialize, Deserialize)]
struct TestRecord {
    // Define the fields based on your table schema
    column1: i32,
}

async fn postgres_test_handler(State(pool_manager) : State<Pool>) -> Json<Vec<TestRecord>>{
    let client = pool_manager.get().await.unwrap();

    let statement = client.prepare_cached("SELECT * FROM public.test_table").await.unwrap();

    let rows = client.query(&statement, &[]).await.unwrap();

    let mut records = Vec::new();

    for row in rows.iter() {
        let mut record = TestRecord {
            column1: 0, // Initialize with default values
            // ...
        };

        for (index, column) in row.columns().iter().enumerate() {
            match column.name() {
                "column1" => record.column1 = row.get(index),
                // Set other fields based on their column names
                // ...
                _ => (),
            }
        }

        records.push(record)
    }

    Json(records)
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
