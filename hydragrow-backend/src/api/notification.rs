use crate::AppState;
use crate::db::postgres::{delete_fcm_token, upsert_fcm_token};
use actix_web::{HttpResponse, Responder, web};
use serde::Deserialize;
use tracing::{error, info};

#[derive(Deserialize)]
pub struct RegisterTokenReq {
    pub fcm_token: String,
    pub device_id: String,
}

#[derive(Deserialize)]
pub struct UnregisterTokenReq {
    pub fcm_token: String,
    pub device_id: String,
}

/// POST /api/notifications/register
pub async fn register_token(
    req: web::Json<RegisterTokenReq>,
    state: web::Data<AppState>,
) -> impl Responder {
    // 1. Persist vào DB
    if let Err(e) = upsert_fcm_token(&state.pg_pool, &req.device_id, &req.fcm_token).await {
        error!(device_id = %req.device_id, error = %e, "Lỗi persist FCM token vào DB");
        return HttpResponse::InternalServerError()
            .json(serde_json::json!({"status": "error", "message": "DB error"}));
    }

    // 2. Cập nhật in-memory cache
    let mut tokens = match state.fcm_tokens.lock() {
        Ok(guard) => guard,
        Err(poisoned) => poisoned.into_inner(),
    };
    let entry = tokens.entry(req.device_id.clone()).or_default();
    if !entry.contains(&req.fcm_token) {
        entry.push(req.fcm_token.clone());
        info!(device_id = %req.device_id, "📱 Registered new FCM token for device");
    }

    HttpResponse::Ok().json(serde_json::json!({"status": "success"}))
}

/// DELETE /api/notifications/unregister
pub async fn unregister_token(
    req: web::Json<UnregisterTokenReq>,
    state: web::Data<AppState>,
) -> impl Responder {
    // 1. Xóa khỏi DB
    if let Err(e) = delete_fcm_token(&state.pg_pool, &req.device_id, &req.fcm_token).await {
        error!(device_id = %req.device_id, error = %e, "Lỗi xóa FCM token khỏi DB");
        return HttpResponse::InternalServerError().json(serde_json::json!({"status": "error"}));
    }

    // 2. Xóa khỏi in-memory cache
    let mut tokens = match state.fcm_tokens.lock() {
        Ok(guard) => guard,
        Err(poisoned) => poisoned.into_inner(),
    };
    if let Some(device_tokens) = tokens.get_mut(&req.device_id) {
        device_tokens.retain(|t| t != &req.fcm_token);
    }

    HttpResponse::Ok().json(serde_json::json!({"status": "success"}))
}

pub fn init_routes(cfg: &mut web::ServiceConfig) {
    cfg.route("/notifications/register", web::post().to(register_token));
    cfg.route(
        "/notifications/unregister",
        web::delete().to(unregister_token),
    );
}
