use crate::AppState;
use actix_web::{
    Error, HttpMessage, HttpResponse,
    body::EitherBody,
    dev::{Service, ServiceRequest, ServiceResponse, Transform, forward_ready},
};
use futures_util::future::{LocalBoxFuture, Ready, ready};
use std::rc::Rc;
use tracing::{debug, error};

#[derive(Clone, Debug, Default)]
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
            None => {
                let response = HttpResponse::InternalServerError()
                    .json(serde_json::json!({"error": "AppState missing"}))
                    .map_into_right_body();
                let (http_req, _payload) = req.into_parts();
                return Box::pin(ready(Ok(ServiceResponse::new(http_req, response))));
            }
        };

        // Local browser inspection only: authenticate against one explicitly configured
        // active user without weakening Firebase/service authentication in other environments.
        if let Some(dev_token) = req
            .headers()
            .get("X-Dev-Auth-Token")
            .and_then(|value| value.to_str().ok())
        {
            let expected_token = std::env::var("DEV_AUTH_TOKEN").unwrap_or_default();
            let configured_user_id = std::env::var("DEV_AUTH_USER_ID")
                .ok()
                .and_then(|value| value.parse::<i64>().ok());
            let is_development = std::env::var("ENVIRONMENT")
                .map(|value| value.eq_ignore_ascii_case("development"))
                .unwrap_or(false);
            let is_loopback = req.peer_addr().is_some_and(|addr| addr.ip().is_loopback());

            if !is_development
                || !is_loopback
                || expected_token.is_empty()
                || configured_user_id.is_none()
                || dev_token != expected_token
            {
                let response = HttpResponse::Unauthorized()
                    .json(serde_json::json!({"error": "Invalid local development authentication"}))
                    .map_into_right_body();
                let (http_req, _payload) = req.into_parts();
                return Box::pin(ready(Ok(ServiceResponse::new(http_req, response))));
            }

            let user_id = configured_user_id.expect("checked above");
            let srv = Rc::clone(&self.service);
            return Box::pin(async move {
                match crate::db::users::find_active_by_id(&app_state.pg_pool, user_id).await {
                    Ok(Some(user)) => {
                        req.extensions_mut().insert(AuthContext {
                            scopes: user.scopes,
                            user_id: Some(user.id.to_string()),
                            session_id: Some(format!("dev-local:{}", user.id)),
                            service_key_label: None,
                        });
                        let res = srv.call(req).await?;
                        Ok(res.map_into_left_body())
                    }
                    Ok(None) => {
                        let response = HttpResponse::Unauthorized()
                            .json(serde_json::json!({"error": "Local development user is not active"}))
                            .map_into_right_body();
                        let (http_req, _payload) = req.into_parts();
                        Ok(ServiceResponse::new(http_req, response))
                    }
                    Err(_) => {
                        let response = HttpResponse::InternalServerError()
                            .json(serde_json::json!({"error": "Local development authentication lookup failed"}))
                            .map_into_right_body();
                        let (http_req, _payload) = req.into_parts();
                        Ok(ServiceResponse::new(http_req, response))
                    }
                }
            });
        }

        // 2. Ưu tiên xác thực bằng Firebase ID token (Authorization: Bearer <token>)
        if req.path().contains("/webhook/")
            && let Some(token) = req
                .headers()
                .get("X-Webhook-Token")
                .and_then(|value| value.to_str().ok())
        {
            let token = token.to_string();
            let device_id = req.match_info().get("device_id").map(str::to_string);
            let srv = Rc::clone(&self.service);
            return Box::pin(async move {
                let key_hash = crate::api::webhook_tokens::sha256_hex(&token);
                if let Some(webhook) =
                    crate::api::webhook_tokens::find_by_token_hash(&app_state.pg_pool, &key_hash)
                        .await
                    && webhook.is_active
                    && device_id.as_deref() == Some(webhook.device_id.as_str())
                {
                    let auth_context = AuthContext {
                        scopes: vec!["webhook:invoke".to_string()],
                        user_id: None,
                        session_id: None,
                        service_key_label: Some(format!("webhook:{}", webhook.id)),
                    };
                    req.extensions_mut().insert(auth_context);
                    let res = srv.call(req).await?;
                    return Ok(res.map_into_left_body());
                }
                let response = HttpResponse::Unauthorized()
                    .json(serde_json::json!({"error": "Invalid or missing Webhook Token"}))
                    .map_into_right_body();
                let (http_req, _payload) = req.into_parts();
                Ok(ServiceResponse::new(http_req, response))
            });
        }

        // 2b. Firebase ID token (Authorization: Bearer <token>)
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
                            service_key_label: None,
                        };
                        req.extensions_mut().insert(auth_context);
                        let res = srv.call(req).await?;
                        Ok(res.map_into_left_body())
                    }
                    Ok(None) => {
                        // Self-registration: user Firebase hợp lệ nhưng chưa có trong
                        // bảng users -> tự tạo với scope đọc mặc định (read:telemetry).
                        // Không ghi đè scope của tài khoản đã tồn tại.
                        let email = claims.email.clone().unwrap_or_default();
                        match crate::db::users::provision_default_user(
                            &app_state.pg_pool,
                            &claims.sub,
                            &email,
                        )
                        .await
                        {
                            Ok(user) => {
                                debug!(firebase_uid = %user.firebase_uid, "Tự cấp tài khoản mới sau đăng ký");
                                let auth_context = AuthContext {
                                    scopes: user.scopes,
                                    user_id: Some(user.id.to_string()),
                                    session_id: Some(claims.sub),
                                    service_key_label: None,
                                };
                                req.extensions_mut().insert(auth_context);
                                let res = srv.call(req).await?;
                                Ok(res.map_into_left_body())
                            }
                            Err(e) => {
                                error!(?e, "Không thể tự cấp tài khoản mới sau đăng ký");
                                let response = HttpResponse::InternalServerError()
                                    .json(serde_json::json!({
                                        "error": "Lỗi hệ thống khi tự cấp tài khoản"
                                    }))
                                    .map_into_right_body();
                                let (http_req, _payload) = req.into_parts();
                                Ok(ServiceResponse::new(http_req, response))
                            }
                        }
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

        // 3. Fallback: X-API-Key — service_api_keys first, legacy shared key second.
        let expected_api_key = app_state.api_key.clone();
        let pg_pool = app_state.pg_pool.clone();

        let header_key = req
            .headers()
            .get("X-API-Key")
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

            let scopes = default_legacy_scopes_for_ws();

            let auth_context = AuthContext {
                scopes,
                user_id: None,
                session_id: None,
                service_key_label: Some("legacy-api-key".to_string()),
            };

            req.extensions_mut().insert(auth_context);

            let res = srv.call(req).await?;
            Ok(res.map_into_left_body())
        })
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum PrincipalKind {
    User,
    Service,
}

impl AuthContext {
    pub fn principal_kind(&self) -> PrincipalKind {
        if self.user_id.is_some() {
            PrincipalKind::User
        } else {
            PrincipalKind::Service
        }
    }
}

pub async fn authorize_device(
    req: &actix_web::HttpRequest,
    app_state: &AppState,
    capability: Option<&str>,
    device_id: &str,
) -> Result<AuthContext, HttpResponse> {
    let auth = req
        .extensions()
        .get::<AuthContext>()
        .cloned()
        .ok_or_else(|| {
            HttpResponse::Unauthorized().json(serde_json::json!({"error": "Unauthorized"}))
        })?;

    if let Some(required) = capability
        && !auth.has_scope(required)
    {
        return Err(HttpResponse::Forbidden().json(serde_json::json!({
            "error": "Missing required scope",
            "required_scope": required
        })));
    }

    if auth.principal_kind() == PrincipalKind::User {
        let user_id = auth
            .user_id
            .as_deref()
            .and_then(|id| id.parse::<i64>().ok())
            .ok_or_else(|| {
                HttpResponse::Unauthorized().json(serde_json::json!({"error": "Unauthorized"}))
            })?;
        let owned = crate::db::device_ownership::is_owner(&app_state.pg_pool, user_id, device_id)
            .await
            .map_err(|_| {
                HttpResponse::InternalServerError()
                    .json(serde_json::json!({"error": "Authorization lookup failed"}))
            })?;
        if !owned {
            return Err(HttpResponse::Forbidden()
                .json(serde_json::json!({"error": "Device ownership required"})));
        }
    }

    Ok(auth)
}

pub async fn authorize_all_devices(
    req: &actix_web::HttpRequest,
    app_state: &AppState,
    capability: Option<&str>,
    device_ids: &[&str],
) -> Result<AuthContext, HttpResponse> {
    let auth = req
        .extensions()
        .get::<AuthContext>()
        .cloned()
        .ok_or_else(|| {
            HttpResponse::Unauthorized().json(serde_json::json!({"error": "Unauthorized"}))
        })?;

    if let Some(required) = capability
        && !auth.has_scope(required)
    {
        return Err(HttpResponse::Forbidden().json(serde_json::json!({
            "error": "Missing required scope",
            "required_scope": required
        })));
    }

    if auth.principal_kind() == PrincipalKind::User {
        let user_id = auth
            .user_id
            .as_deref()
            .and_then(|id| id.parse::<i64>().ok())
            .ok_or_else(|| {
                HttpResponse::Unauthorized().json(serde_json::json!({"error": "Unauthorized"}))
            })?;
        let owned =
            crate::db::device_ownership::is_owner_of_all(&app_state.pg_pool, user_id, device_ids)
                .await
                .map_err(|_| {
                    HttpResponse::InternalServerError()
                        .json(serde_json::json!({"error": "Authorization lookup failed"}))
                })?;
        if !owned {
            return Err(HttpResponse::Forbidden().json(serde_json::json!({
                "error": "Device ownership required for all targets"
            })));
        }
    }

    Ok(auth)
}

/// Enforces ownership for every `/api/devices/{device_id}/...` REST route.
/// Service principals are authorized by their service scopes; user principals
/// must also own the target device. This is deliberately separate from the
/// route-specific capability checks.
pub struct DeviceOwnershipAuth;

impl<S, B> Transform<S, ServiceRequest> for DeviceOwnershipAuth
where
    S: Service<ServiceRequest, Response = ServiceResponse<B>, Error = Error> + 'static,
    S::Future: 'static,
    B: 'static,
{
    type Response = ServiceResponse<EitherBody<B>>;
    type Error = Error;
    type InitError = ();
    type Transform = DeviceOwnershipAuthMiddleware<S>;
    type Future = Ready<Result<Self::Transform, Self::InitError>>;

    fn new_transform(&self, service: S) -> Self::Future {
        ready(Ok(DeviceOwnershipAuthMiddleware {
            service: Rc::new(service),
        }))
    }
}

pub struct DeviceOwnershipAuthMiddleware<S> {
    service: Rc<S>,
}

impl<S, B> Service<ServiceRequest> for DeviceOwnershipAuthMiddleware<S>
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
        let srv = Rc::clone(&self.service);
        let device_id = req.match_info().get("device_id").map(str::to_owned);
        let app_state = req.app_data::<actix_web::web::Data<AppState>>().cloned();
        let auth = req.extensions().get::<AuthContext>().cloned();

        Box::pin(async move {
            let Some(device_id) = device_id else {
                let res = srv.call(req).await?;
                return Ok(res.map_into_left_body());
            };
            let Some(auth) = auth else {
                let (http_req, _payload) = req.into_parts();
                let response = HttpResponse::Unauthorized()
                    .json(serde_json::json!({"error": "Unauthorized"}))
                    .map_into_right_body();
                return Ok(ServiceResponse::new(http_req, response));
            };
            let Some(app_state) = app_state else {
                let (http_req, _payload) = req.into_parts();
                let response = HttpResponse::InternalServerError()
                    .json(serde_json::json!({"error": "AppState missing"}))
                    .map_into_right_body();
                return Ok(ServiceResponse::new(http_req, response));
            };

            if auth.principal_kind() == PrincipalKind::User {
                let user_id = match auth
                    .user_id
                    .as_deref()
                    .and_then(|id| id.parse::<i64>().ok())
                {
                    Some(id) => id,
                    None => {
                        let (http_req, _payload) = req.into_parts();
                        let response = HttpResponse::Unauthorized()
                            .json(serde_json::json!({"error": "Unauthorized"}))
                            .map_into_right_body();
                        return Ok(ServiceResponse::new(http_req, response));
                    }
                };
                match crate::db::device_ownership::is_owner(&app_state.pg_pool, user_id, &device_id)
                    .await
                {
                    Ok(true) => {}
                    Ok(false) => {
                        let (http_req, _payload) = req.into_parts();
                        let response = HttpResponse::Forbidden()
                            .json(serde_json::json!({"error": "Device ownership required"}))
                            .map_into_right_body();
                        return Ok(ServiceResponse::new(http_req, response));
                    }
                    Err(_) => {
                        let (http_req, _payload) = req.into_parts();
                        let response = HttpResponse::InternalServerError()
                            .json(serde_json::json!({"error": "Authorization lookup failed"}))
                            .map_into_right_body();
                        return Ok(ServiceResponse::new(http_req, response));
                    }
                }
            }

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

pub(crate) fn default_legacy_scopes_for_ws() -> Vec<String> {
    vec![
        "read:telemetry".to_string(),
        "write:config".to_string(),
        "control:pump".to_string(),
        "control:emergency".to_string(),
        "device:ota".to_string(),
        "device:network".to_string(),
        "health:read".to_string(),
        "webhook:invoke".to_string(),
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

    #[test]
    fn legacy_service_context_is_not_user_scoped() {
        let auth = AuthContext {
            scopes: default_legacy_scopes_for_ws(),
            user_id: None,
            session_id: None,
            service_key_label: Some("legacy-api-key".to_string()),
        };

        assert_eq!(auth.principal_kind(), PrincipalKind::Service);
        assert!(auth.user_id.is_none());
    }

    #[test]
    fn firebase_user_context_is_user_scoped() {
        let auth = AuthContext {
            scopes: vec!["read:telemetry".to_string()],
            user_id: Some("42".to_string()),
            session_id: Some("firebase-session".to_string()),
            service_key_label: None,
        };

        assert_eq!(auth.principal_kind(), PrincipalKind::User);
    }
}
