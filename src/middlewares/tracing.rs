use axum::{
    response::Response,
    http::Request,
    middleware::Next
};
use tracing::info;
use std::time::Instant;
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
    
    //Log tracing information
    info!(
        method = %method,
        url = %url,
        headers = %headers,
        res_status = %res_status,
        request_time = %request_time,
        "Request processed"
    );

    //println!("{} {} => {} ({}ms)", method, url, res_status, request_time);
    
    response
}

pub fn init_tracing_with_jaeger() -> Result<(), Box<dyn std::error::Error>> {
    global::set_text_map_propagator(opentelemetry_jaeger::Propagator::new());
    // Sets up the machinery needed to export data to Jaeger
    let tracer = opentelemetry_jaeger::new_pipeline()
        .with_service_name("Proxy_cache_aws")
        .install_simple()?;

    // Create a tracing layer with the configured tracer
    let opentelemetry = tracing_opentelemetry::layer().with_tracer(tracer);

    // The SubscriberExt and SubscriberInitExt traits are needed to extend the
    // Registry to accept `opentelemetry (the OpenTelemetryLayer type).
    tracing_subscriber::registry()
        .with(opentelemetry)
        // Log to stdout in JSON format
        .with(fmt::Layer::new().json())
        .try_init()?;

    Ok(())
}


#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_init_tracing_with_jaeger() {
        let result = init_tracing_with_jaeger();
        assert!(result.is_ok());
    }
}