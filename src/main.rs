pub mod middlewares;
pub mod facades;
use facades::s3::S3Facade;

use axum::{
    response::Html, 
    routing::get,
    Router,
    middleware,
    extract::State,
};
use middlewares::tracing::tracing_fn;
use std::{env, net::SocketAddr};


#[derive(Clone)]
pub struct Config {
    pub secret: String,
}

fn create_addr() -> SocketAddr {
    let host = env::var("APP_HOST").unwrap_or("0.0.0.0".to_string());
    let port = env::var("APP_PORT").unwrap_or("5000".to_string());
    let addr_str = format!("{}:{}", host, port);
    addr_str.parse().unwrap_or_else(|_| panic!("{} is not a valid addr", addr_str))
}

#[tokio::main]
async fn main() {
    let secret_test = Config {secret: "olo".to_string()};

    // build our application with a route
    let app = Router::new()
        .route("/", get(handler))
        .route("/secret", get(say_secret))
        .layer(middleware::from_fn(tracing_fn))
        .with_state(secret_test);

    //Create app url
    let addr = create_addr();

    // run it
    println!("listening on {}", addr);
    axum::Server::bind(&addr)
        .serve(app.into_make_service())
        .await
        .unwrap();
}

async fn handler() -> Html<&'static str> {
    Html("<h1>Hello, World!</h1>")
}

async fn say_secret(State(config) : State<Config>) -> String {
    return config.secret;
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::env;

    #[test]
    fn create_valid_addr() {
        env::set_var("APP_HOST", "127.0.0.1");
        env::set_var("APP_PORT", "5000");
        assert_eq!(create_addr().to_string(), "127.0.0.1:5000".to_string());
    }
    #[test]
    fn create_invalid_addr() {
        env::set_var("APP_HOST", "asd2.111.222.333");
        env::set_var("APP_PORT", "5000");
        assert!(std::panic::catch_unwind(||create_addr()).is_err())
    }
}
