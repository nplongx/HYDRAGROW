// src/api/notification.rs
use crate::AppState;
use actix_web::{HttpResponse, Responder, web};
use serde::Deserialize;
use tracing::info;

#[derive(Deserialize)]
pub struct RegisterTokenReq {
    pub fcm_token: String,
}

// API: POST /api/notifications/register
pub async fn register_token(
    req: web::Json<RegisterTokenReq>,
    state: web::Data<AppState>,
) -> impl Responder {
    let mut tokens = state.fcm_tokens.lock().unwrap();

    // Nếu token chưa có trong RAM thì thêm vào
    if !tokens.contains(&req.fcm_token) {
        tokens.push(req.fcm_token.clone());
        info!("📱 Đã lưu FCM Token mới của thiết bị Android!");
    }

    HttpResponse::Ok().json(serde_json::json!({"status": "success"}))
}

pub fn init_routes(cfg: &mut web::ServiceConfig) {
    cfg.route("/notifications/register", web::post().to(register_token));
}
