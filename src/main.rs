mod middlewares;
mod facades;

use axum::{
    response::Html, 
    routing::get,
    Router,
    middleware,
    extract::State,
};
use std::{env, net::SocketAddr};
use middlewares::tracing::tracing_middleware;

#[derive(Clone)]
struct Config {
    secret: String,
}

fn create_addr() -> SocketAddr {
    let host = env::var("APP_HOST").unwrap_or("127.0.0.1".to_string());
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
        .layer(middleware::from_fn(tracing_middleware))
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