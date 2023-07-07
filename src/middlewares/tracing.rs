use axum::{
    response::Response,
    http::Request,
    middleware::Next
};
use std::time::Instant;
use opentelemetry::{global, KeyValue, trace::{Span, Tracer}};
use std::convert::Infallible;

pub async fn tracing_fn<B>(request: Request<B>, next: Next<B>) -> Result<Response, Infallible> {
    //Extract necessary information from the request
    let method = request.method().to_string();
    let url = request.uri().to_string();

    // Start a new Span for this request
    let tracer = global::tracer("my-component-test");
    let mut span = tracer.start(format!("{} {}", method, url));

    //Add key-value pairs to the span
    span.set_attribute(KeyValue::new("http.method", method.clone()));
    span.set_attribute(KeyValue::new("http.url", url.clone()));

    //Start the timer
    let start = Instant::now();
    
    //Execute the next middleware/request
    let response = next.run(request).await;

    //The request time (in ms)
    let request_time = start.elapsed().as_millis().to_string();

    //Add more attributes to the span
    span.set_attribute(KeyValue::new("http.status_code", response.status().to_string()));
    span.set_attribute(KeyValue::new("http.request_time_ms", request_time.clone()));

   //Print tracing information
   println!("{} {} => {} ({}ms)", method, url, response.status().to_string(), request_time);

   // End the Span when the request is done
   span.end();

   Ok(response)
}