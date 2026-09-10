// hydragrow-backend/src/api/admin_users.rs
//! Provisioning tài khoản: gán scope nội bộ cho một Firebase UID đã được
//! admin tạo thủ công trên Firebase Console. Không có tự đăng ký.
//!
//! Endpoint này khoá bằng chính `X-API-Key` gốc (không dùng qua Firebase Bearer
//! token), vì mục đích của nó là chính là tạo/cập nhật các tài khoản đó.

use actix_web::{HttpMessage, HttpRequest, HttpResponse, Responder, web};
use serde::{Deserialize, Serialize};
use tracing::warn;

use crate::AppState;
use crate::api::middleware::auth::AuthContext;
use crate::db::users;

const VALID_ROLES: [&str; 3] = ["admin", "operator", "viewer"];

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

#[derive(Debug, Deserialize)]
pub struct UpdateUserRequest {
    pub role: Option<String>,
    pub scopes: Option<Vec<String>>,
    pub is_active: Option<bool>,
}

#[derive(Debug, Deserialize)]
pub struct UpdatePreferencesRequest {
    pub preferences: serde_json::Value,
}

fn is_root_api_key(req: &HttpRequest, app_state: &AppState) -> bool {
    req.headers()
        .get("X-API-Key")
        .and_then(|v| v.to_str().ok())
        .is_some_and(|key| key == app_state.api_key)
}

/// Admin = root API key HOẶC scope `device:admin` / `*`.
fn is_admin(req: &HttpRequest, app_state: &AppState) -> bool {
    if is_root_api_key(req, app_state) {
        return true;
    }
    req.extensions()
        .get::<AuthContext>()
        .is_some_and(|ctx| ctx.has_scope("device:admin"))
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
    cfg.route("/admin/users", web::get().to(list_users));
    cfg.route("/admin/users/{id}", web::patch().to(update_user));
    cfg.route("/admin/whoami", web::get().to(whoami));
    cfg.route(
        "/admin/me/preferences",
        web::put().to(update_my_preferences),
    );
}

/// GET /api/admin/users — danh sách mọi user (role, scopes, is_active).
pub async fn list_users(req: HttpRequest, app_state: web::Data<AppState>) -> impl Responder {
    if !is_admin(&req, &app_state) {
        warn!("Từ chối list_users: thiếu quyền admin");
        return HttpResponse::Forbidden()
            .json(serde_json::json!({"error": "Cần quyền admin để xem danh sách user"}));
    }

    match users::list_users(&app_state.pg_pool).await {
        Ok(rows) => HttpResponse::Ok().json(serde_json::json!({"status": "success", "data": rows})),
        Err(e) => {
            tracing::error!(?e, "Lỗi khi list_users");
            HttpResponse::InternalServerError()
                .json(serde_json::json!({"error": "Không thể đọc danh sách user"}))
        }
    }
}

/// GET /api/admin/whoami — trả role + scopes của user đang đăng nhập (Bearer token).
pub async fn whoami(req: HttpRequest, app_state: web::Data<AppState>) -> impl Responder {
    let ctx = req
        .extensions()
        .get::<AuthContext>()
        .cloned()
        .unwrap_or_default();

    let firebase_uid = match ctx.session_id {
        Some(uid) if !uid.is_empty() => uid,
        _ => {
            return HttpResponse::Unauthorized()
                .json(serde_json::json!({"error": "Cần đăng nhập bằng Bearer token"}));
        }
    };

    match users::find_active_by_firebase_uid(&app_state.pg_pool, &firebase_uid).await {
        Ok(Some(user)) => {
            HttpResponse::Ok().json(serde_json::json!({"status": "success", "data": user}))
        }
        Ok(None) => {
            HttpResponse::Unauthorized().json(serde_json::json!({"error": "Không tìm thấy user"}))
        }
        Err(e) => {
            tracing::error!(?e, "Lỗi khi whoami");
            HttpResponse::InternalServerError()
                .json(serde_json::json!({"error": "Lỗi hệ thống khi xác thực"}))
        }
    }
}

/// PUT /api/admin/me/preferences — cập nhật preferences của chính user đang login.
pub async fn update_my_preferences(
    req: HttpRequest,
    app_state: web::Data<AppState>,
    body: web::Json<UpdatePreferencesRequest>,
) -> impl Responder {
    let ctx = req
        .extensions()
        .get::<AuthContext>()
        .cloned()
        .unwrap_or_default();

    let firebase_uid = match ctx.session_id {
        Some(uid) if !uid.is_empty() => uid,
        _ => {
            return HttpResponse::Unauthorized()
                .json(serde_json::json!({"error": "Cần đăng nhập bằng Bearer token"}));
        }
    };

    match users::update_user_preferences(&app_state.pg_pool, &firebase_uid, &body.preferences).await
    {
        Ok(Some(user)) => {
            HttpResponse::Ok().json(serde_json::json!({"status": "success", "data": user}))
        }
        Ok(None) => {
            HttpResponse::NotFound().json(serde_json::json!({"error": "Không tìm thấy user"}))
        }
        Err(e) => {
            tracing::error!(?e, "Lỗi khi update_user_preferences");
            HttpResponse::InternalServerError()
                .json(serde_json::json!({"error": "Không thể lưu tùy chọn người dùng"}))
        }
    }
}

/// PATCH /api/admin/users/{id} — đổi role / scopes / is_active.
pub async fn update_user(
    req: HttpRequest,
    app_state: web::Data<AppState>,
    path: web::Path<i64>,
    body: web::Json<UpdateUserRequest>,
) -> impl Responder {
    if !is_admin(&req, &app_state) {
        warn!("Từ chối update_user: thiếu quyền admin");
        return HttpResponse::Forbidden()
            .json(serde_json::json!({"error": "Cần quyền admin để sửa user"}));
    }

    let id = path.into_inner();

    if body.role.is_none() && body.scopes.is_none() && body.is_active.is_none() {
        return HttpResponse::BadRequest().json(
            serde_json::json!({"error": "Cần ít nhất một field: role, scopes hoặc is_active"}),
        );
    }

    if body
        .role
        .as_deref()
        .is_some_and(|r| !VALID_ROLES.contains(&r))
    {
        return HttpResponse::BadRequest().json(serde_json::json!({
            "error": format!("role phải là một trong: {}", VALID_ROLES.join(", "))
        }));
    }

    match users::update_user(
        &app_state.pg_pool,
        id,
        body.role.as_deref(),
        body.scopes.as_deref(),
        body.is_active,
    )
    .await
    {
        Ok(Some(user)) => {
            HttpResponse::Ok().json(serde_json::json!({"status": "success", "data": user}))
        }
        Ok(None) => HttpResponse::NotFound()
            .json(serde_json::json!({"error": format!("Không tìm thấy user id={id}")})),
        Err(e) => {
            tracing::error!(?e, "Lỗi khi update_user");
            HttpResponse::InternalServerError()
                .json(serde_json::json!({"error": "Không thể lưu user"}))
        }
    }
}
