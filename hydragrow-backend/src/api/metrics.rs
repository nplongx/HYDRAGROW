// src/api/metrics.rs

use actix_web::{HttpRequest, HttpResponse, Responder};
use std::env;

use crate::metrics::gather_metrics;

pub async fn metrics_handler(req: HttpRequest) -> impl Responder {
    // Token bí mật riêng dành cho Grafana Cloud scrape.
    let expected_token = match env::var("METRICS_TOKEN") {
        Ok(token) if !token.is_empty() => token,
        _ => {
            return HttpResponse::InternalServerError().body("METRICS_TOKEN is not configured");
        }
    };

    // Đọc:
    // Authorization: Bearer <token>
    let authorized = req
        .headers()
        .get("Authorization")
        .and_then(|value| value.to_str().ok())
        .and_then(|value| value.strip_prefix("Bearer "))
        .is_some_and(|token| token == expected_token);

    if !authorized {
        return HttpResponse::Unauthorized()
            .insert_header(("WWW-Authenticate", "Bearer"))
            .body("Unauthorized");
    }

    let output = gather_metrics();

    HttpResponse::Ok()
        .insert_header(("Content-Type", "text/plain; version=0.0.4; charset=utf-8"))
        .body(output)
}
