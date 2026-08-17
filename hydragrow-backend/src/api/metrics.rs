// src/api/metrics.rs
use crate::metrics::gather_metrics;
use actix_web::{HttpResponse, Responder};

pub async fn metrics_handler() -> impl Responder {
    let output = gather_metrics();
    HttpResponse::Ok()
        .content_type("text/plain; version=0.0.4; charset=utf-8")
        .body(output)
}
