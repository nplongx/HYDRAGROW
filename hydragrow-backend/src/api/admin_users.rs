// hydragrow-backend/src/api/admin_users.rs
//! Provisioning tài khoản: gán scope nội bộ cho một Firebase UID đã được
//! admin tạo thủ công trên Firebase Console. Không có tự đăng ký.
//!
//! Endpoint này khoá bằng chính `X-API-Key` gốc (không dùng qua Firebase Bearer
//! token), vì mục đích của nó là chính là tạo/cập nhật các tài khoản đó.

use actix_web::{HttpRequest, HttpResponse, Responder, web};
use serde::{Deserialize, Serialize};
use tracing::warn;

use crate::AppState;
use crate::db::users;

#[derive(Debug, Deserialize)]
pub struct ProvisionUserRequest {
    pub firebase_uid: String,
    pub email: String,
    pub display_name: Option<String>,
    pub scopes: Vec<String>,
}

#[derive(Debug, Serialize)]
pub struct ProvisionUserResponse {
    pub id: i64,
    pub firebase_uid: String,
    pub email: String,
    pub scopes: Vec<String>,
}

fn is_root_api_key(req: &HttpRequest, app_state: &AppState) -> bool {
    req.headers()
        .get("X-API-Key")
        .and_then(|v| v.to_str().ok())
        .is_some_and(|key| key == app_state.api_key)
}

pub async fn provision_user(
    req: HttpRequest,
    app_state: web::Data<AppState>,
    body: web::Json<ProvisionUserRequest>,
) -> impl Responder {
    if !is_root_api_key(&req, &app_state) {
        warn!("Từ chối provision_user: thiếu/sai X-API-Key gốc");
        return HttpResponse::Forbidden()
            .json(serde_json::json!({"error": "Cần X-API-Key gốc để cấp tài khoản"}));
    }

    if body.firebase_uid.trim().is_empty() || body.email.trim().is_empty() {
        return HttpResponse::BadRequest()
            .json(serde_json::json!({"error": "firebase_uid và email không được để trống"}));
    }

    match users::upsert_user(
        &app_state.pg_pool,
        body.firebase_uid.trim(),
        body.email.trim(),
        body.display_name.as_deref(),
        &body.scopes,
    )
    .await
    {
        Ok(user) => HttpResponse::Ok().json(ProvisionUserResponse {
            id: user.id,
            firebase_uid: user.firebase_uid,
            email: user.email,
            scopes: user.scopes,
        }),
        Err(e) => {
            tracing::error!(?e, "Lỗi upsert user khi provisioning");
            HttpResponse::InternalServerError()
                .json(serde_json::json!({"error": "Không thể lưu tài khoản"}))
        }
    }
}

pub fn init_routes(cfg: &mut web::ServiceConfig) {
    cfg.route("/admin/users", web::post().to(provision_user));
}
