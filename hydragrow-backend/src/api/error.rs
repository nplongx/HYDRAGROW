use actix_web::{
    Error, HttpResponse,
    body::MessageBody,
    dev::{Service, ServiceRequest, ServiceResponse, Transform, forward_ready},
    http::StatusCode,
};
use futures_util::future::{LocalBoxFuture, Ready, ready};
use serde::{Deserialize, Serialize};
use serde_json::{Map, Value, json};
use std::rc::Rc;

const MAX_MESSAGE_LEN: usize = 512;
const MAX_DETAILS_BYTES: usize = 8192;
const MAX_REQUEST_ID_LEN: usize = 128;

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct ApiErrorBody {
    pub code: String,
    pub message: String,
    pub details: Value,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub request_id: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct ApiErrorEnvelope {
    pub error: ApiErrorBody,
}

pub fn status_code_for(status: StatusCode) -> &'static str {
    match status {
        StatusCode::BAD_REQUEST => "invalid_request",
        StatusCode::UNAUTHORIZED => "unauthenticated",
        StatusCode::FORBIDDEN => "forbidden",
        StatusCode::NOT_FOUND => "not_found",
        StatusCode::CONFLICT => "conflict",
        StatusCode::UNPROCESSABLE_ENTITY => "invalid_operation",
        StatusCode::TOO_MANY_REQUESTS => "rate_limited",
        StatusCode::BAD_GATEWAY | StatusCode::SERVICE_UNAVAILABLE => "dependency_unavailable",
        StatusCode::GATEWAY_TIMEOUT => "dependency_timeout",
        _ => "internal_server_error",
    }
}

fn bound_message(message: &str) -> String {
    message.chars().take(MAX_MESSAGE_LEN).collect()
}

fn bounded_request_id(value: Option<&str>) -> Option<String> {
    value
        .filter(|v| !v.is_empty())
        .map(|v| v.chars().take(MAX_REQUEST_ID_LEN).collect())
}

fn bound_details(value: Value) -> Value {
    if serde_json::to_vec(&value).is_ok_and(|bytes| bytes.len() <= MAX_DETAILS_BYTES) {
        value
    } else {
        json!({ "truncated": true })
    }
}

fn is_canonical(value: &Value) -> bool {
    value
        .get("error")
        .and_then(Value::as_object)
        .is_some_and(|error| {
            error.get("code").and_then(Value::as_str).is_some()
                && error.get("message").and_then(Value::as_str).is_some()
                && error.get("details").is_some()
        })
}

fn safe_message(status: StatusCode, message: &str) -> String {
    let message = match status {
        StatusCode::BAD_GATEWAY
        | StatusCode::SERVICE_UNAVAILABLE
        | StatusCode::GATEWAY_TIMEOUT
        | StatusCode::INTERNAL_SERVER_ERROR => match status {
            StatusCode::BAD_GATEWAY => "Dependency unavailable",
            StatusCode::SERVICE_UNAVAILABLE => "Service unavailable",
            StatusCode::GATEWAY_TIMEOUT => "Dependency timeout",
            _ => "Internal server error",
        },
        _ => message,
    };
    bound_message(message)
}

pub fn normalize_error_json(status: StatusCode, value: Value, request_id: Option<&str>) -> Value {
    if is_canonical(&value) {
        let Some(error) = value.get("error").and_then(Value::as_object) else {
            unreachable!("is_canonical guarantees an error object");
        };
        return json!({
            "error": {
                "code": error.get("code").and_then(Value::as_str).unwrap_or_else(|| status_code_for(status)),
                "message": safe_message(status, error.get("message").and_then(Value::as_str).unwrap_or("Request failed")),
                "details": if status.is_server_error() { json!({}) } else { bound_details(error.get("details").cloned().unwrap_or_else(|| json!({}))) },
                "request_id": bounded_request_id(error.get("request_id").and_then(Value::as_str).or(request_id)),
            }
        });
    }

    let Some(object) = value.as_object() else {
        return json!({
            "error": {
                "code": status_code_for(status),
                "message": safe_message(status, value.as_str().unwrap_or("Request failed")),
                "details": {},
                "request_id": bounded_request_id(request_id),
            }
        });
    };

    let message = object
        .get("message")
        .or_else(|| object.get("error"))
        .and_then(Value::as_str)
        .unwrap_or("Request failed");
    let code = object
        .get("code")
        .and_then(Value::as_str)
        .filter(|code| !code.is_empty())
        .unwrap_or_else(|| status_code_for(status));

    let mut details = Map::new();
    for (key, val) in object {
        if !matches!(key.as_str(), "error" | "message" | "code" | "request_id") {
            details.insert(key.clone(), val.clone());
        }
    }
    let request_id = object
        .get("request_id")
        .and_then(Value::as_str)
        .or(request_id);

    json!({
        "error": {
            "code": code,
            "message": safe_message(status, message),
            "details": if status.is_server_error() { json!({}) } else { bound_details(Value::Object(details)) },
            "request_id": bounded_request_id(request_id),
        }
    })
}

pub struct NormalizeJsonErrors;

impl<S, B> Transform<S, ServiceRequest> for NormalizeJsonErrors
where
    S: Service<ServiceRequest, Response = ServiceResponse<B>, Error = Error> + 'static,
    S::Future: 'static,
    B: MessageBody + 'static,
{
    type Response = ServiceResponse;
    type Error = Error;
    type InitError = ();
    type Transform = NormalizeJsonErrorsMiddleware<S>;
    type Future = Ready<Result<Self::Transform, Self::InitError>>;

    fn new_transform(&self, service: S) -> Self::Future {
        ready(Ok(NormalizeJsonErrorsMiddleware {
            service: Rc::new(service),
        }))
    }
}

pub struct NormalizeJsonErrorsMiddleware<S> {
    service: Rc<S>,
}

impl<S, B> Service<ServiceRequest> for NormalizeJsonErrorsMiddleware<S>
where
    S: Service<ServiceRequest, Response = ServiceResponse<B>, Error = Error> + 'static,
    S::Future: 'static,
    B: MessageBody + 'static,
{
    type Response = ServiceResponse;
    type Error = Error;
    type Future = LocalBoxFuture<'static, Result<Self::Response, Self::Error>>;

    forward_ready!(service);

    fn call(&self, req: ServiceRequest) -> Self::Future {
        let srv = Rc::clone(&self.service);
        Box::pin(async move {
            let request_id = req
                .headers()
                .get("X-Request-Id")
                .and_then(|v| v.to_str().ok())
                .map(str::to_owned);
            let response = srv.call(req).await?;
            let status = response.status();
            let is_json = response
                .headers()
                .get(actix_web::http::header::CONTENT_TYPE)
                .and_then(|v| v.to_str().ok())
                .is_some_and(|v| v.starts_with("application/json"));

            if status.is_client_error() || status.is_server_error() {
                if is_json {
                    let (request, response) = response.into_parts();
                    let body = actix_web::body::to_bytes(response.into_body())
                        .await
                        .map_err(|_| {
                            actix_web::error::ErrorInternalServerError("response body read failed")
                        })?;
                    let value =
                        serde_json::from_slice::<Value>(&body).unwrap_or_else(|_| json!({}));
                    let normalized = normalize_error_json(status, value, request_id.as_deref());
                    let response = HttpResponse::build(status)
                        .json(normalized)
                        .map_into_boxed_body();
                    return Ok(ServiceResponse::new(request, response));
                }
            }
            Ok(response.map_into_boxed_body())
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use actix_web::{App, Responder, test as actix_test, web};

    #[test]
    fn normalizes_legacy_error_string_to_canonical_envelope() {
        let value = normalize_error_json(
            StatusCode::FORBIDDEN,
            json!({"error":"Missing required scope","required_scope":"read:telemetry"}),
            Some("req-1"),
        );
        assert_eq!(value["error"]["code"], "forbidden");
        assert_eq!(value["error"]["message"], "Missing required scope");
        assert_eq!(
            value["error"]["details"]["required_scope"],
            "read:telemetry"
        );
        assert_eq!(value["error"]["request_id"], "req-1");
    }

    #[test]
    fn preserves_existing_machine_code() {
        let value = normalize_error_json(
            StatusCode::CONFLICT,
            json!({"error":"recipe_not_found","code":"recipe_not_found","message":"missing"}),
            None,
        );
        assert_eq!(value["error"]["code"], "recipe_not_found");
        assert_eq!(value["error"]["details"], json!({}));
    }

    #[test]
    fn maps_statuses_to_stable_codes() {
        assert_eq!(status_code_for(StatusCode::BAD_REQUEST), "invalid_request");
        assert_eq!(status_code_for(StatusCode::UNAUTHORIZED), "unauthenticated");
        assert_eq!(status_code_for(StatusCode::FORBIDDEN), "forbidden");
        assert_eq!(status_code_for(StatusCode::NOT_FOUND), "not_found");
        assert_eq!(status_code_for(StatusCode::CONFLICT), "conflict");
        assert_eq!(
            status_code_for(StatusCode::TOO_MANY_REQUESTS),
            "rate_limited"
        );
        assert_eq!(
            status_code_for(StatusCode::SERVICE_UNAVAILABLE),
            "dependency_unavailable"
        );
        assert_eq!(
            status_code_for(StatusCode::GATEWAY_TIMEOUT),
            "dependency_timeout"
        );
        assert_eq!(
            status_code_for(StatusCode::INTERNAL_SERVER_ERROR),
            "internal_server_error"
        );
    }

    #[test]
    fn bounds_message_and_details() {
        let long = "x".repeat(MAX_MESSAGE_LEN + 100);
        let value = normalize_error_json(StatusCode::BAD_REQUEST, json!({"error": long}), None);
        assert_eq!(
            value["error"]["message"].as_str().unwrap().len(),
            MAX_MESSAGE_LEN
        );

        let huge = json!({"error": "bad", "payload": "x".repeat(MAX_DETAILS_BYTES + 100)});
        let value = normalize_error_json(StatusCode::BAD_REQUEST, huge, None);
        assert_eq!(value["error"]["details"]["truncated"], true);
    }

    #[test]
    fn server_errors_scrub_internal_message_and_details() {
        let value = normalize_error_json(
            StatusCode::INTERNAL_SERVER_ERROR,
            json!({"error":"SQL error: password=secret","details":"connection string"}),
            None,
        );
        assert_eq!(value["error"]["message"], "Internal server error");
        assert_eq!(value["error"]["details"], json!({}));
    }

    #[actix_web::test]
    async fn middleware_normalizes_json_error_response_and_preserves_status() {
        async fn legacy_error() -> impl Responder {
            HttpResponse::Forbidden().json(json!({
                "error": "Missing required scope",
                "required_scope": "device:admin"
            }))
        }

        let app = actix_test::init_service(
            App::new()
                .wrap(NormalizeJsonErrors)
                .route("/test", web::get().to(legacy_error)),
        )
        .await;
        let req = actix_test::TestRequest::get()
            .uri("/test")
            .insert_header(("X-Request-Id", "req-42"))
            .to_request();
        let response = actix_test::call_service(&app, req).await;
        assert_eq!(response.status(), StatusCode::FORBIDDEN);
        let body: Value = actix_test::read_body_json(response).await;
        assert_eq!(body["error"]["code"], "forbidden");
        assert_eq!(body["error"]["details"]["required_scope"], "device:admin");
        assert_eq!(body["error"]["request_id"], "req-42");
    }
}
