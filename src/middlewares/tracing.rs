use axum::{
    response::Response,
    http::Request,
    middleware::Next
};
use tracing::info;
use std::{time::Instant, env};
use opentelemetry::global;
use tracing_subscriber::{
    fmt, layer::SubscriberExt, util::SubscriberInitExt,
};

pub async fn tracing_fn<B>(request: Request<B>, next: Next<B>) -> Response {
    //Extract necessary information from the request
    let method = request.method().to_string();
    let url = request.uri().to_string();
    let headers = format!("{:?}", request.headers());
    
    //Start the timer
    let start = Instant::now();
    
    //Execute the next middleware/request
    let response = next.run(request).await;
    
    //Extract necessary information from the response
    let res_status = response.status().to_string();
    
    //The request time (in ms)
    let request_time = start.elapsed().as_millis().to_string();
    
    // uncomment in production
    //Log tracing information
    if env::var("WITH_LOGS").map(|v| v == "true").unwrap_or(true) {
        info!(
            method = %method,
            url = %url,
            headers = %headers,
            res_status = %res_status,
            request_time = %request_time,
            "Request processed"
        );
    } else {
       // println!("{} {} => {} ({}ms)", method, url, res_status, request_time);
    }

    
    response
}

pub fn init_tracing() -> Result<(), Box<dyn std::error::Error>> {
    // Create a tracing layer that logs to stdout in JSON format
    let json_layer = fmt::Layer::new().json();

    // Initialize the tracing subscriber with the JSON layer
    tracing_subscriber::registry().with(json_layer).try_init()?;

    Ok(())
}