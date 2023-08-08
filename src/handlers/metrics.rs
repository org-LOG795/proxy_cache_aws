use axum::response::{Response, IntoResponse};
use axum::body::Full;
use prometheus::{register_int_counter_vec, IntCounterVec, Encoder, TextEncoder};

pub async fn handle_metrics() -> impl IntoResponse {
    let encoder = TextEncoder::new();
    let metric_families = prometheus::gather();
    let mut buffer = vec![];
    encoder.encode(&metric_families, &mut buffer).unwrap();

    let response = Response::builder()
        .header("Content-Type", encoder.format_type())
        .body(Full::from(buffer))
        .unwrap();

    response
}