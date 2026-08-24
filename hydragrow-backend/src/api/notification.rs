// src/api/notification.rs
use crate::AppState;
use actix_web::{HttpResponse, Responder, web};
use serde::Deserialize;
use tracing::info;

#[derive(Deserialize)]
pub struct RegisterTokenReq {
    pub fcm_token: String,
    pub device_id: String,
}

// API: POST /api/notifications/register
pub async fn register_token(
    req: web::Json<RegisterTokenReq>,
    state: web::Data<AppState>,
) -> impl Responder {
    let mut tokens = match state.fcm_tokens.lock() {
        Ok(guard) => guard,
        Err(poisoned) => poisoned.into_inner(),
    };
    let entry = tokens.entry(req.device_id.clone()).or_insert_with(Vec::new);

    if !entry.contains(&req.fcm_token) {
        entry.push(req.fcm_token.clone());
        info!(device_id = %req.device_id, "📱 Registered new FCM token for device");
    }

    HttpResponse::Ok().json(serde_json::json!({"status": "success"}))
}

pub fn init_routes(cfg: &mut web::ServiceConfig) {
    cfg.route("/notifications/register", web::post().to(register_token));
}
