use crate::AppState;
use actix_web::{
    Error, HttpMessage, HttpResponse,
    body::EitherBody,
    dev::{Service, ServiceRequest, ServiceResponse, Transform, forward_ready},
};
use futures_util::future::{LocalBoxFuture, Ready, ready};
use std::rc::Rc;
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

        let app_state = req.app_data::<actix_web::web::Data<AppState>>().unwrap();
        let expected_api_key = &app_state.api_key;

        // 2. Thử lấy từ Header trước (Dành cho API bình thường)
        let header_key = req
            .headers()
            .get("X-API-Key") // Nếu bạn dùng Bearer Token, hãy đổi thành "Authorization"
            .and_then(|hv| hv.to_str().ok());

        // Cấu trúc phụ để bóc tách query string nhanh gọn
        #[derive(serde::Deserialize)]
        struct QueryAuth {
            api_key: Option<String>,
        }

        let mut is_authorized = false;

        // Kiểm tra Header trước
        if let Some(key) = header_key {
            // Giả định bạn truyền header đúng như mong đợi.
            // Nếu bạn dùng chuỗi "Bearer <token>" thì phải cắt chuỗi ra nhé.
            if key == expected_api_key {
                is_authorized = true;
            }
        }
        // 3. Nếu Header không có, thử tìm trong Query String (Dành cho WebSocket)
        else if let Ok(query) = actix_web::web::Query::<QueryAuth>::from_query(req.query_string())
        {
            if let Some(key) = &query.api_key {
                if key == expected_api_key {
                    is_authorized = true;
                }
            }
        }

        // Nếu cả 2 cách đều thất bại -> Trả về 401
        if !is_authorized {
            let response = HttpResponse::Unauthorized()
                .json(serde_json::json!({"error": "Unauthorized: Invalid or missing API Key"}))
                .map_into_right_body();
            let (http_req, _payload) = req.into_parts();
            return Box::pin(ready(Ok(ServiceResponse::new(http_req, response))));
        }

        // Cho phép request đi tiếp tới route handlers
        let srv = Rc::clone(&self.service);
        Box::pin(async move {
            let res = srv.call(req).await?;
            Ok(res.map_into_left_body())
        })
    }
}
