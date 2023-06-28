mod middlewares;
mod facades;

use axum::{
    body::Body,
    http::Request,
    response::Html, 
    routing::get,
    Router,
    middleware,
    extract::State,
};
use opentelemetry::{
    global::shutdown_tracer_provider,
    sdk::Resource,
    trace::TraceError,
    global, 
    sdk::trace as sdktrace,
    trace::Tracer,
};
use opentelemetry_otlp::WithExportConfig;
use opentelemetry_http::HeaderExtractor;

use std::{env, net::SocketAddr,error::Error};
use middlewares::tracing::tracing_middleware;


#[derive(Clone)]
struct Config {
    secret: String,
}

fn init_tracer() -> Result<sdktrace::Tracer, TraceError> {
    opentelemetry_otlp::new_pipeline()
        .tracing()
        .with_exporter(opentelemetry_otlp::new_exporter().tonic().with_env())
        .with_trace_config(
            sdktrace::config().with_resource(Resource::default()),
        )
        .install_batch(opentelemetry::runtime::Tokio)
}

fn create_addr() -> SocketAddr {
    let host = env::var("APP_HOST").unwrap_or("0.0.0.0".to_string());
    let port = env::var("APP_PORT").unwrap_or("5000".to_string());
    let addr_str = format!("{}:{}", host, port);
    addr_str.parse().unwrap_or_else(|_| panic!("{} is not a valid addr", addr_str))
}
#[tokio::main]
async fn main() -> Result<(), Box<dyn Error + Send + Sync + 'static>> {
    let _ = init_tracer()?;
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

    shutdown_tracer_provider();
    Ok(())
}


async fn handler(req: Request<Body>) -> Html<&'static str> {
    //let parent_cx = global::get_text_map_propagator(|propagator| {
       // propagator.extract(&HeaderExtractor(req.headers()))
    //});
    //tracer.start_with_context("context_name", &parent_cx);
    Html("<h1>Hello, World!</h1>")
}

async fn say_secret(State(config) : State<Config>) -> String {
    return config.secret;
}