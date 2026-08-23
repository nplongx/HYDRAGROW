use crate::AppState;
use actix_web::{
    Error, HttpMessage, HttpResponse,
    body::EitherBody,
    dev::{Service, ServiceRequest, ServiceResponse, Transform, forward_ready},
};
use futures_util::future::{LocalBoxFuture, Ready, ready};
use std::rc::Rc;
use tracing::error;

#[derive(Clone, Debug, Default)]
pub struct AuthContext {
    pub scopes: Vec<String>,
    pub user_id: Option<String>,
    pub session_id: Option<String>,
}

impl AuthContext {
    pub fn has_scope(&self, scope: &str) -> bool {
        self.scopes.iter().any(|s| s == scope || s == "*")
    }
}

pub struct ApiKeyAuth;

impl ApiKeyAuth {
    pub fn new() -> Self {
        Self {}
    }
}

impl Default for ApiKeyAuth {
    fn default() -> Self {
        Self::new()
    }
}

impl<S, B> Transform<S, ServiceRequest> for ApiKeyAuth
where
    S: Service<ServiceRequest, Response = ServiceResponse<B>, Error = Error> + 'static,
    S::Future: 'static,
    B: 'static,
{
    type Response = ServiceResponse<EitherBody<B>>;
    type Error = Error;
    type InitError = ();
    type Transform = ApiKeyAuthMiddleware<S>;
    type Future = Ready<Result<Self::Transform, Self::InitError>>;

    fn new_transform(&self, service: S) -> Self::Future {
        ready(Ok(ApiKeyAuthMiddleware {
            service: Rc::new(service),
        }))
    }
}

pub struct ApiKeyAuthMiddleware<S> {
    service: Rc<S>,
}

impl<S, B> Service<ServiceRequest> for ApiKeyAuthMiddleware<S>
where
    S: Service<ServiceRequest, Response = ServiceResponse<B>, Error = Error> + 'static,
    S::Future: 'static,
    B: 'static,
{
    type Response = ServiceResponse<EitherBody<B>>;
    type Error = Error;
    type Future = LocalBoxFuture<'static, Result<Self::Response, Self::Error>>;

    forward_ready!(service);

    fn call(&self, req: ServiceRequest) -> Self::Future {
        // 1. Bypass cho OPTIONS request (CORS Preflight)
        if req.method() == actix_web::http::Method::OPTIONS {
            let srv = Rc::clone(&self.service);
            return Box::pin(async move {
                let res = srv.call(req).await?;
                Ok(res.map_into_left_body())
            });
        }

        // Bypass cho WebSocket
        if req.path() == "/metrics" || req.path().ends_with("/ws") {
            let srv = Rc::clone(&self.service);
            return Box::pin(async move {
                let res = srv.call(req).await?;
                Ok(res.map_into_left_body())
            });
        }

        let app_state = match req.app_data::<actix_web::web::Data<AppState>>() {
            Some(state) => state.clone(),
            None => return Box::pin(ready(Err(actix_web::error::ErrorUnauthorized("Missing app state")))),
        };

        // 2. Ưu tiên xác thực bằng Firebase ID token (Authorization: Bearer <token>)
        if let Some(token) = extract_bearer_token(req.headers()) {
            let token = token.to_string();
            let srv = Rc::clone(&self.service);
            return Box::pin(async move {
                let claims = match app_state.firebase_auth.verify(&token).await {
                    Ok(claims) => claims,
                    Err(e) => {
                        let response = HttpResponse::Unauthorized()
                            .json(serde_json::json!({
                                "error": format!("Token không hợp lệ: {e}")
                            }))
                            .map_into_right_body();
                        let (http_req, _payload) = req.into_parts();
                        return Ok(ServiceResponse::new(http_req, response));
                    }
                };

                match crate::db::users::find_active_by_firebase_uid(&app_state.pg_pool, &claims.sub)
                    .await
                {
                    Ok(Some(user)) => {
                        let auth_context = AuthContext {
                            scopes: user.scopes,
                            user_id: Some(user.id.to_string()),
                            session_id: Some(claims.sub),
                        };
                        req.extensions_mut().insert(auth_context);
                        let res = srv.call(req).await?;
                        Ok(res.map_into_left_body())
                    }
                    Ok(None) => {
                        let response = HttpResponse::Forbidden()
                            .json(serde_json::json!({
                                "error": "Tài khoản chưa được cấp quyền truy cập"
                            }))
                            .map_into_right_body();
                        let (http_req, _payload) = req.into_parts();
                        Ok(ServiceResponse::new(http_req, response))
                    }
                    Err(e) => {
                        error!(?e, "Lỗi truy vấn user theo firebase_uid");
                        let response = HttpResponse::InternalServerError()
                            .json(serde_json::json!({
                                "error": "Lỗi hệ thống khi xác thực"
                            }))
                            .map_into_right_body();
                        let (http_req, _payload) = req.into_parts();
                        Ok(ServiceResponse::new(http_req, response))
                    }
                }
            });
        }

        // 3. Fallback: X-API-Key tĩnh (đường cũ, giữ để tương thích ngược)
        let expected_api_key = app_state.api_key.clone();

        let header_key = req
            .headers()
            .get("X-API-Key")
            .and_then(|hv| hv.to_str().ok())
            .map(ToString::to_string);

        let is_authorized = header_key
            .as_deref()
            .is_some_and(|key| key == expected_api_key);

        if !is_authorized {
            let response = HttpResponse::Unauthorized()
                .json(serde_json::json!({"error": "Unauthorized: Invalid or missing API Key"}))
                .map_into_right_body();
            let (http_req, _payload) = req.into_parts();
            return Box::pin(ready(Ok(ServiceResponse::new(http_req, response))));
        }

        let scopes = default_legacy_scopes();

        let auth_context = AuthContext {
            scopes,
            user_id: req
                .headers()
                .get("X-User-Id")
                .and_then(|hv| hv.to_str().ok())
                .map(ToString::to_string),
            session_id: req
                .headers()
                .get("X-Session-Id")
                .and_then(|hv| hv.to_str().ok())
                .map(ToString::to_string),
        };

        req.extensions_mut().insert(auth_context);

        let srv = Rc::clone(&self.service);
        Box::pin(async move {
            let res = srv.call(req).await?;
            Ok(res.map_into_left_body())
        })
    }
}

fn extract_bearer_token(headers: &actix_web::http::header::HeaderMap) -> Option<&str> {
    headers
        .get(actix_web::http::header::AUTHORIZATION)
        .and_then(|value| value.to_str().ok())
        .and_then(|value| value.strip_prefix("Bearer "))
        .map(str::trim)
        .filter(|token| !token.is_empty())
}

fn default_legacy_scopes() -> Vec<String> {
    vec![
        "read:telemetry".to_string(),
        "write:config".to_string(),
        "control:pump".to_string(),
        "control:emergency".to_string(),
        "device:ota".to_string(),
        "device:network".to_string(),
    ]
}

#[cfg(test)]
mod tests {
    use super::*;
    use actix_web::http::header::{AUTHORIZATION, HeaderMap, HeaderValue};

    #[test]
    fn extracts_token_from_valid_bearer_header() {
        let mut headers = HeaderMap::new();
        headers.insert(
            AUTHORIZATION,
            HeaderValue::from_static("Bearer abc.def.ghi"),
        );
        assert_eq!(extract_bearer_token(&headers), Some("abc.def.ghi"));
    }

    #[test]
    fn returns_none_when_no_bearer_prefix() {
        let mut headers = HeaderMap::new();
        headers.insert(AUTHORIZATION, HeaderValue::from_static("Basic abc123"));
        assert_eq!(extract_bearer_token(&headers), None);
    }

    #[test]
    fn returns_none_when_header_missing() {
        let headers = HeaderMap::new();
        assert_eq!(extract_bearer_token(&headers), None);
    }

    #[test]
    fn returns_none_for_empty_token() {
        let mut headers = HeaderMap::new();
        headers.insert(AUTHORIZATION, HeaderValue::from_static("Bearer "));
        assert_eq!(extract_bearer_token(&headers), None);
    }
}
