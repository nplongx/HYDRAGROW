use actix_web::{
    Error, HttpMessage, HttpResponse, Responder,
    body::MessageBody,
    dev::{Service, ServiceRequest, ServiceResponse, Transform, forward_ready},
    http::header::{HeaderName, HeaderValue},
    web,
};
use futures_util::future::{LocalBoxFuture, Ready, ready};
use std::{
    sync::{
        Arc,
        atomic::{AtomicBool, Ordering},
    },
    time::Instant,
};
use tracing::info_span;
use uuid::Uuid;

use crate::{AppState, metrics};

const REQUEST_ID: &str = "x-request-id";
const CORRELATION_ID: &str = "x-correlation-id";
const TRACEPARENT: &str = "traceparent";
const MAX_ID_LEN: usize = 128;

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct RequestContext {
    pub request_id: String,
    pub correlation_id: String,
    pub trace_id: Option<String>,
}

fn valid_id(value: &str) -> bool {
    !value.is_empty()
        && value.len() <= MAX_ID_LEN
        && value
            .bytes()
            .all(|b| b.is_ascii_alphanumeric() || matches!(b, b'.' | b'_' | b':' | b'-'))
}

fn incoming_id(req: &ServiceRequest, name: &str) -> Option<String> {
    req.headers()
        .get(name)
        .and_then(|v| v.to_str().ok())
        .filter(|v| valid_id(v))
        .map(str::to_owned)
}

fn new_id() -> String {
    Uuid::new_v4().to_string()
}

fn trace_id_from_traceparent(value: &str) -> Option<String> {
    let mut parts = value.split('-');
    let version = parts.next()?;
    let trace_id = parts.next()?;
    let span_id = parts.next()?;
    let flags = parts.next()?;
    if version.len() != 2 || trace_id.len() != 32 || span_id.len() != 16 || flags.len() != 2 {
        return None;
    }
    if !trace_id.bytes().all(|b| b.is_ascii_hexdigit())
        || !span_id.bytes().all(|b| b.is_ascii_hexdigit())
        || !flags.bytes().all(|b| b.is_ascii_hexdigit())
    {
        return None;
    }
    Some(trace_id.to_ascii_lowercase())
}

pub fn request_context(req: &ServiceRequest) -> RequestContext {
    let request_id = incoming_id(req, REQUEST_ID).unwrap_or_else(new_id);
    let correlation_id = incoming_id(req, CORRELATION_ID).unwrap_or_else(|| request_id.clone());
    let trace_id = req
        .headers()
        .get(TRACEPARENT)
        .and_then(|v| v.to_str().ok())
        .and_then(trace_id_from_traceparent);
    RequestContext {
        request_id,
        correlation_id,
        trace_id,
    }
}

#[derive(Clone, Copy, Debug, Default)]
pub struct RequestObservability;

impl<S, B> Transform<S, ServiceRequest> for RequestObservability
where
    S: Service<ServiceRequest, Response = ServiceResponse<B>, Error = Error> + 'static,
    S::Future: 'static,
    B: MessageBody + 'static,
{
    type Response = ServiceResponse<B>;
    type Error = Error;
    type InitError = ();
    type Transform = RequestObservabilityMiddleware<S>;
    type Future = Ready<Result<Self::Transform, Self::InitError>>;

    fn new_transform(&self, service: S) -> Self::Future {
        ready(Ok(RequestObservabilityMiddleware {
            service: std::rc::Rc::new(service),
        }))
    }
}

pub struct RequestObservabilityMiddleware<S> {
    service: std::rc::Rc<S>,
}

impl<S, B> Service<ServiceRequest> for RequestObservabilityMiddleware<S>
where
    S: Service<ServiceRequest, Response = ServiceResponse<B>, Error = Error> + 'static,
    S::Future: 'static,
    B: MessageBody + 'static,
{
    type Response = ServiceResponse<B>;
    type Error = Error;
    type Future = LocalBoxFuture<'static, Result<Self::Response, Self::Error>>;

    forward_ready!(service);

    fn call(&self, req: ServiceRequest) -> Self::Future {
        let ctx = request_context(&req);
        let method = req.method().to_string();
        let path = req.path().to_string();
        let endpoint = req
            .match_pattern()
            .unwrap_or_else(|| "unmatched".to_string())
            .to_string();
        let span = info_span!(
            "http.request",
            service = "hydragrow-backend",
            component = "http",
            event_type = "request",
            method = %method,
            endpoint = %endpoint,
            request_id = %ctx.request_id,
            correlation_id = %ctx.correlation_id,
            trace_id = ctx.trace_id.as_deref().unwrap_or(""),
        );
        req.extensions_mut().insert(ctx.clone());
        let start = Instant::now();
        let request_id = ctx.request_id.clone();
        let correlation_id = ctx.correlation_id.clone();
        let future = self.service.call(req);

        Box::pin(async move {
            let result = {
                let _entered = span.enter();
                future.await
            };
            let elapsed = start.elapsed();
            match result {
                Ok(mut response) => {
                    let status = response.status();
                    metrics::HTTP_REQUESTS_TOTAL
                        .with_label_values(&[method.as_str(), endpoint.as_str(), status.as_str()])
                        .inc();
                    metrics::HTTP_REQ_DURATION_SECONDS
                        .with_label_values(&[method.as_str(), endpoint.as_str()])
                        .observe(elapsed.as_secs_f64());
                    if let Ok(value) = HeaderValue::from_str(&request_id) {
                        response
                            .headers_mut()
                            .insert(HeaderName::from_static(REQUEST_ID), value);
                    }
                    if let Ok(value) = HeaderValue::from_str(&correlation_id) {
                        response
                            .headers_mut()
                            .insert(HeaderName::from_static(CORRELATION_ID), value);
                    }
                    Ok(response)
                }
                Err(error) => {
                    metrics::HTTP_REQUESTS_TOTAL
                        .with_label_values(&[method.as_str(), path.as_str(), "500"])
                        .inc();
                    metrics::HTTP_REQ_DURATION_SECONDS
                        .with_label_values(&[method.as_str(), endpoint.as_str()])
                        .observe(elapsed.as_secs_f64());
                    Err(error)
                }
            }
        })
    }
}

pub async fn liveness() -> impl Responder {
    HttpResponse::Ok().json(serde_json::json!({"status":"ok"}))
}

pub async fn readiness(state: web::Data<AppState>) -> impl Responder {
    let postgres = sqlx::query_scalar::<_, i32>("SELECT 1")
        .fetch_one(&state.pg_pool)
        .await
        .is_ok();
    let influx = state.influx_client.health().await.is_ok();
    let mqtt = state.mqtt_connected.load(Ordering::Relaxed);
    let event_bus = true;
    let configuration_sync_worker = "not_configured";
    let command_reconciliation_worker = state.command_reconciliation_worker.load(Ordering::Relaxed);
    let ready = postgres && influx && mqtt && event_bus && command_reconciliation_worker;
    let body = serde_json::json!({
        "status": if ready { "ready" } else { "not_ready" },
        "dependencies": {
            "postgresql": postgres,
            "influxdb": influx,
            "mqtt": mqtt,
            "event_websocket_fanout": event_bus,
            "configuration_synchronization_worker": configuration_sync_worker,
            "command_reconciliation_worker": command_reconciliation_worker
        }
    });
    if ready {
        HttpResponse::Ok().json(body)
    } else {
        HttpResponse::ServiceUnavailable().json(body)
    }
}

pub fn init_routes(cfg: &mut web::ServiceConfig) {
    cfg.route("/livez", web::get().to(liveness));
    cfg.route("/readyz", web::get().to(readiness));
}

pub fn new_mqtt_connection_state() -> Arc<AtomicBool> {
    Arc::new(AtomicBool::new(false))
}

#[cfg(test)]
mod tests {
    use super::*;
    use actix_web::test as actix_test;

    #[test]
    fn invalid_correlation_is_replaced_and_valid_correlation_is_preserved() {
        let req = actix_test::TestRequest::get()
            .insert_header((CORRELATION_ID, "bad id"))
            .insert_header((REQUEST_ID, "req-1"))
            .to_srv_request();
        let ctx = request_context(&req);
        assert_eq!(ctx.request_id, "req-1");
        assert_ne!(ctx.correlation_id, "bad id");

        let req = actix_test::TestRequest::get()
            .insert_header((CORRELATION_ID, "corr-1"))
            .to_srv_request();
        assert_eq!(request_context(&req).correlation_id, "corr-1");
    }

    #[test]
    fn traceparent_extracts_only_valid_w3c_trace_id() {
        let value = "00-4bf92f3577b34da6a3ce929d0e0e4736-00f067aa0ba902b7-01";
        assert_eq!(
            trace_id_from_traceparent(value),
            Some("4bf92f3577b34da6a3ce929d0e0e4736".to_string())
        );
        assert!(trace_id_from_traceparent("not-a-trace").is_none());
    }

    #[test]
    fn generated_correlation_is_request_stable() {
        let req = actix_test::TestRequest::get().to_srv_request();
        let ctx = request_context(&req);
        assert_eq!(ctx.request_id, ctx.correlation_id);
        assert!(valid_id(&ctx.request_id));
    }
}
