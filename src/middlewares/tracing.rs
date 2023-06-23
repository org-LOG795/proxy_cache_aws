use axum::{
    response::Response,
    http::Request,
    middleware::Next
};
use std::time::Instant;

pub async fn tracing_middleware<B>(request: Request<B>, next: Next<B>) -> Response {
    //Extract necessary information from the request
    let method = request.method().to_string();
    let url = request.uri().to_string();

    //Start the timer
    let start = Instant::now();
    
    //Execute the next middleware/request
    let response = next.run(request).await;

    //The request time (in ms)
    let request_time = start.elapsed().as_millis().to_string();

    //Extract necessary information from the response
    let res_status = response.status().to_string();

    //Print tracing information
    println!("{} {} => {} ({}ms)", method, url, res_status, request_time);

    return response;
}