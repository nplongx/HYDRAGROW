use crate::AppState;
use actix_web::{
    Error, HttpResponse,
    body::EitherBody,
    dev::{Service, ServiceRequest, ServiceResponse, Transform, forward_ready},
};
use futures_util::future::{LocalBoxFuture, Ready, ready};
use std::rc::Rc;

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
// use std::sync::Arc; // Bỏ comment nếu bạn thực sự dùng Arc ở đâu đó

pub struct ApiKeyAuth {
    // Không cần lưu api_key ở struct Transform nữa vì chúng ta đang lấy từ AppState
    // api_key: String,
}

impl ApiKeyAuth {
    // Cập nhật hàm new cho phù hợp
    pub fn new() -> Self {
        Self {}
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
        // --- 1. BYPASS OPTIONS REQUEST CHO CORS ---
        // Trình duyệt gửi preflight OPTIONS request và nó KHÔNG CÓ header auth
        // Chúng ta phải cho phép nó đi qua để Actix-cors xử lý phần còn lại
        if req.method() == actix_web::http::Method::OPTIONS {
            let srv = Rc::clone(&self.service);
            return Box::pin(async move {
                let res = srv.call(req).await?;
                Ok(res.map_into_left_body())
            });
        }
        // ------------------------------------------

        if req.path().ends_with("/ws") {
            let srv = Rc::clone(&self.service);
            return Box::pin(async move {
                let res = srv.call(req).await?;
                Ok(res.map_into_left_body())
            });
        }

        let app_state = req.app_data::<actix_web::web::Data<AppState>>().unwrap();
        let expected_api_key = &app_state.api_key;

        // 2. Thử lấy từ Header trước (Dành cho API bình thường)
        let header_key = req
            .headers()
            .get("X-API-Key") // Nếu bạn dùng Bearer Token, hãy đổi thành "Authorization"
            .and_then(|hv| hv.to_str().ok());

        let is_authorized = header_key.is_some_and(|key| key == expected_api_key);

        // Nếu cả 2 cách đều thất bại -> Trả về 401
        if !is_authorized {
            let response = HttpResponse::Unauthorized()
                .json(serde_json::json!({"error": "Unauthorized: Invalid or missing API Key"}))
                .map_into_right_body();
            let (http_req, _payload) = req.into_parts();
            return Box::pin(ready(Ok(ServiceResponse::new(http_req, response))));
        }

        auth_context.user_id = req
            .headers()
            .get("X-User-Id")
            .and_then(|hv| hv.to_str().ok())
            .map(ToString::to_string);
        auth_context.session_id = req
            .headers()
            .get("X-Session-Id")
            .and_then(|hv| hv.to_str().ok())
            .map(ToString::to_string);
        req.extensions_mut().insert(auth_context);

        // Cho phép request đi tiếp tới route handlers
        let srv = Rc::clone(&self.service);
        Box::pin(async move {
            let res = srv.call(req).await?;
            Ok(res.map_into_left_body())
        })
    }
}

fn parse_scopes(raw: Option<&str>) -> Option<Vec<String>> {
    let scopes: Vec<String> = raw?
        .split(|c| c == ',' || c == ' ')
        .map(str::trim)
        .filter(|s| !s.is_empty())
        .map(ToString::to_string)
        .collect();
    if scopes.is_empty() {
        None
    } else {
        Some(scopes)
    }
}

fn default_legacy_scopes() -> Vec<String> {
    vec![
        "read:telemetry".to_string(),
        "write:config".to_string(),
        "control:pump".to_string(),
        "control:emergency".to_string(),
    ]
}
