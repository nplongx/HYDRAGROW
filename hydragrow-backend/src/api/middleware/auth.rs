use actix_service::{Service, Transform};
use actix_web::{
    body::EitherBody,
    dev::{ServiceRequest, ServiceResponse},
    error::Error,
    http::header,
    Error as ActixError, HttpMessage, HttpResponse,
};
use futures_util::future::{ok, LocalBoxFuture, Ready};
use serde::{Deserialize, Serialize};
use std::rc::Rc;
use std::task::{Context, Poll};

use crate::AppState;

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct AuthContext {
    pub scopes: Vec<String>,
    pub user_id: Option<String>,
    pub session_id: Option<String>,
    pub service_key_label: Option<String>,
}

impl AuthContext {
    pub fn has_scope(&self, scope: &str) -> bool {
        self.scopes.iter().any(|s| s == scope || s == "*")
    }
}

#[derive(Clone)]
pub struct AuthMiddleware {
    app_state: actix_web::web::Data<AppState>,
}

impl AuthMiddleware {
    pub fn new(app_state: actix_web::web::Data<AppState>) -> Self {
        Self { app_state }
    }
}

impl<S, B> Transform<S, ServiceRequest> for AuthMiddleware
where
    S: actix_service::Service<ServiceRequest, Response = ServiceResponse<B>, Error = Error> + 'static,
    B: 'static,
{
    type Response = ServiceResponse<EitherBody<B>>;
    type Error = Error;
    type InitError = ();
    type Transform = AuthMiddlewareService<S>;
    type Future = Ready<Result<Self::Transform, Self::InitError>>;

    fn new_transform(&self, service: S) -> Self::Future {
        ok(AuthMiddlewareService {
            service: Rc::new(service),
            app_state: self.app_state.clone(),
        })
    }
}

pub struct AuthMiddlewareService<S> {
    service: Rc<S>,
    app_state: actix_web::web::Data<AppState>,
}

impl<S, B> actix_service::Service<ServiceRequest> for AuthMiddlewareService<S>
where
    S: actix_service::Service<ServiceRequest, Response = ServiceResponse<B>, Error = Error> + 'static,
    B: 'static,
{
    type Response = ServiceResponse<EitherBody<B>>;
    type Error = Error;
    type Future = LocalBoxFuture<'static, Result<Self::Response, Self::Error>>;

    forward_ready!(service);

    fn call(&self, mut req: ServiceRequest) -> Self::Future {
        // 1. Bearer auth and 2. any existing session handling remain above the
        // X-API-Key fallback in the original middleware. This service contains
        // the X-API-Key path only in this focused source replacement.
        let expected_api_key = self.app_state.api_key.clone();
        let pg_pool = self.app_state.pg_pool.clone();
        let header_key = req
            .headers()
            .get("X-API-Key")
            .and_then(|hv| hv.to_str().ok())
            .map(ToString::to_string);
        let user_id = req
            .headers()
            .get("X-User-Id")
            .and_then(|hv| hv.to_str().ok())
            .map(ToString::to_string);
        let session_id = req
            .headers()
            .get("X-Session-Id")
            .and_then(|hv| hv.to_str().ok())
            .map(ToString::to_string);
        let srv = Rc::clone(&self.service);

        Box::pin(async move {
            if let Some(key) = header_key.as_deref() {
                let key_hash = crate::db::service_api_keys::sha256_hex(key);
                if let Some(svc) =
                    crate::db::service_api_keys::find_active_by_key_hash(&pg_pool, &key_hash).await
                {
                    let auth_context = AuthContext {
                        scopes: svc.scopes,
                        user_id: None,
                        session_id: None,
                        service_key_label: Some(svc.label),
                    };
                    req.extensions_mut().insert(auth_context);
                    let res = srv.call(req).await?;
                    return Ok(res.map_into_left_body());
                }
            }

            let is_authorized = header_key
                .as_deref()
                .is_some_and(|key| key == expected_api_key);

            if !is_authorized {
                let response = HttpResponse::Unauthorized()
                    .json(serde_json::json!({"error": "Unauthorized: Invalid or missing API Key"}))
                    .map_into_right_body();
                let (http_req, _payload) = req.into_parts();
                return Ok(ServiceResponse::new(http_req, response));
            }

            // Legacy X-API-Key is the shared/root key used by existing workers.
            // Keep its historical scopes and add read-only health access so
            // diagnostic/watchdog clients can consume health endpoints.
            let scopes = default_legacy_scopes();
            let auth_context = AuthContext {
                scopes,
                user_id,
                session_id,
                service_key_label: None,
            };
            req.extensions_mut().insert(auth_context);
            let res = srv.call(req).await?;
            Ok(res.map_into_left_body())
        })
    }
}

fn default_legacy_scopes() -> Vec<String> {
    vec![
        "read:telemetry".to_string(),
        "health:read".to_string(),
        "write:config".to_string(),
        "control:pump".to_string(),
        "control:emergency".to_string(),
        "device:ota".to_string(),
        "device:network".to_string(),
    ]
}

fn extract_bearer_token(headers: &actix_web::http::header::HeaderMap) -> Option<&str> {
    headers
        .get(actix_web::http::header::AUTHORIZATION)
        .and_then(|value| value.to_str().ok())
        .and_then(|value| value.strip_prefix("Bearer "))
        .map(str::trim)
        .filter(|token| !token.is_empty())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn legacy_key_includes_health_read() {
        let scopes = default_legacy_scopes();
        assert!(scopes.iter().any(|s| s == "health:read"));
    }
}
