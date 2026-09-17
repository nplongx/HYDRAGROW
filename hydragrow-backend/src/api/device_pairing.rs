use actix_web::{HttpMessage, HttpRequest, HttpResponse, Responder, web};
use serde::{Deserialize, Serialize};

use crate::AppState;
use crate::api::middleware::auth::AuthContext;
use crate::db::device_ownership;

pub fn user_id_from(req: &HttpRequest) -> Option<i64> {
    req.extensions()
        .get::<AuthContext>()
        .and_then(|ctx| ctx.user_id.as_ref())
        .and_then(|id| id.parse::<i64>().ok())
}

#[derive(Debug, Deserialize)]
pub struct ClaimRequest {
    pub device_id: String,
    pub label: Option<String>,
    /// Factory hardware identity (e.g. `esp32c3-<mac>`) proving possession.
    /// Required when the device_id is already hardware-bound.
    pub hardware_id: Option<String>,
}

#[derive(Debug, Serialize)]
pub struct ClaimResponse {
    pub device_id: String,
    pub label: Option<String>,
    pub qr_payload: String, // base URL để mobile scan
    /// Only present the first time this device is ever claimed by anyone.
    /// Show this to the user once — the backend never stores or returns it again.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub mqtt_username: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub mqtt_password: Option<String>,
}

/// POST /api/devices/claim — Gán device_id vào tài khoản đang đăng nhập.
pub async fn claim_device(
    req: HttpRequest,
    app_state: web::Data<AppState>,
    body: web::Json<ClaimRequest>,
) -> impl Responder {
    let Some(user_id) = user_id_from(&req) else {
        return HttpResponse::Unauthorized().json(serde_json::json!({"error": "Chưa đăng nhập"}));
    };
    let device_id = body.device_id.trim().to_string();
    if device_id.is_empty() {
        return HttpResponse::BadRequest()
            .json(serde_json::json!({"error": "device_id không được rỗng"}));
    }

    match device_ownership::claim_device(
        &app_state.pg_pool,
        user_id,
        &device_id,
        body.label.as_deref(),
        body.hardware_id.as_deref(),
    )
    .await
    {
        Ok((rec, mqtt_credentials)) => {
            // QR payload cho mobile scan khi cắm điện lần đầu
            let qr_payload = format!("hydragrow://claim/{}", device_id);
            HttpResponse::Ok().json(ClaimResponse {
                device_id: rec.device_id,
                label: rec.label,
                qr_payload,
                mqtt_username: mqtt_credentials.as_ref().map(|c| c.mqtt_username.clone()),
                mqtt_password: mqtt_credentials.map(|c| c.mqtt_password),
            })
        }
        Err(device_ownership::ClaimError::DuplicateHardware { device_id }) => {
            HttpResponse::Conflict().json(serde_json::json!({
                "error": "device_id already bound to different hardware",
                "device_id": device_id,
            }))
        }
        Err(e) => {
            tracing::error!(?e, "Lỗi claim device");
            HttpResponse::InternalServerError()
                .json(serde_json::json!({"error": "Không thể claim thiết bị"}))
        }
    }
}

/// DELETE /api/devices/{device_id}/claim — Xoá liên kết.
pub async fn unclaim_device(
    req: HttpRequest,
    path: web::Path<String>,
    app_state: web::Data<AppState>,
) -> impl Responder {
    let Some(user_id) = user_id_from(&req) else {
        return HttpResponse::Unauthorized().json(serde_json::json!({"error": "Chưa đăng nhập"}));
    };
    let device_id = path.into_inner();
    match device_ownership::unclaim_device(&app_state.pg_pool, user_id, &device_id).await {
        Ok(0) => {
            HttpResponse::NotFound().json(serde_json::json!({"error": "Không tìm thấy liên kết"}))
        }
        Ok(_) => HttpResponse::Ok()
            .json(serde_json::json!({"status": "unclaimed", "device_id": device_id})),
        Err(e) => {
            tracing::error!(?e, "Lỗi unclaim device");
            HttpResponse::InternalServerError()
                .json(serde_json::json!({"error": "Không thể unclaim"}))
        }
    }
}

/// GET /api/devices — Liệt kê thiết bị của user hiện tại.
pub async fn list_my_devices(req: HttpRequest, app_state: web::Data<AppState>) -> impl Responder {
    let Some(user_id) = user_id_from(&req) else {
        return HttpResponse::Unauthorized().json(serde_json::json!({"error": "Chưa đăng nhập"}));
    };
    match device_ownership::list_devices_for_user(&app_state.pg_pool, user_id).await {
        Ok(devices) => HttpResponse::Ok().json(devices),
        Err(e) => {
            tracing::error!(?e, "Lỗi list devices");
            HttpResponse::InternalServerError()
                .json(serde_json::json!({"error": "Không thể lấy danh sách"}))
        }
    }
}

#[derive(Debug, Deserialize)]
pub struct RenameRequest {
    pub label: Option<String>,
}

/// PATCH /api/devices/{device_id}/label — Đổi tên nhãn thiết bị.
pub async fn rename_device(
    req: HttpRequest,
    path: web::Path<String>,
    app_state: web::Data<AppState>,
    body: web::Json<RenameRequest>,
) -> impl Responder {
    let Some(user_id) = user_id_from(&req) else {
        return HttpResponse::Unauthorized().json(serde_json::json!({"error": "Chưa đăng nhập"}));
    };
    let device_id = path.into_inner();
    let result =
        sqlx::query("UPDATE device_ownership SET label = $1 WHERE user_id = $2 AND device_id = $3")
            .bind(&body.label)
            .bind(user_id)
            .bind(&device_id)
            .execute(&app_state.pg_pool)
            .await;

    match result {
        Ok(r) if r.rows_affected() > 0 => HttpResponse::Ok()
            .json(serde_json::json!({"status": "renamed", "device_id": device_id})),
        Ok(_) => {
            HttpResponse::NotFound().json(serde_json::json!({"error": "Không tìm thấy thiết bị"}))
        }
        Err(e) => {
            tracing::error!(?e, "Lỗi rename device");
            HttpResponse::InternalServerError().json(serde_json::json!({"error": "Lỗi hệ thống"}))
        }
    }
}

/// Guard dùng trong các handler: kiểm tra user hiện tại có sở hữu device_id không.
/// Trả về `Ok(user_id)` nếu có quyền, `Err(HttpResponse)` nếu không.
pub async fn require_device_owner(
    req: &HttpRequest,
    app_state: &web::Data<AppState>,
    device_id: &str,
) -> Result<i64, HttpResponse> {
    let auth = req
        .extensions()
        .get::<AuthContext>()
        .cloned()
        .ok_or_else(|| {
            HttpResponse::Unauthorized().json(serde_json::json!({"error": "Unauthorized"}))
        })?;

    // Service principals are already authorized by ApiKeyAuth scopes and
    // DeviceOwnershipAuth. They do not have a user ownership row to check.
    if auth.user_id.is_none() {
        return Ok(0);
    }

    let user_id = user_id_from(req).ok_or_else(|| {
        HttpResponse::Unauthorized().json(serde_json::json!({"error": "Chưa đăng nhập"}))
    })?;
    let owned = device_ownership::is_owner(&app_state.pg_pool, user_id, device_id)
        .await
        .map_err(|_| {
            HttpResponse::InternalServerError()
                .json(serde_json::json!({"error": "Authorization lookup failed"}))
        })?;
    if owned {
        Ok(user_id)
    } else {
        Err(HttpResponse::Forbidden().json(serde_json::json!({
            "error": "Bạn không có quyền truy cập thiết bị này"
        })))
    }
}

pub fn init_routes(cfg: &mut web::ServiceConfig) {
    cfg.route("/devices", web::get().to(list_my_devices))
        .route("/devices/claim", web::post().to(claim_device))
        .route(
            "/devices/{device_id}/claim",
            web::delete().to(unclaim_device),
        )
        .route("/devices/{device_id}/label", web::patch().to(rename_device));
}

#[cfg(test)]
mod tests {
    use super::*;

    #[actix_web::test]
    async fn service_principal_can_pass_device_guard() {
        let req = actix_web::test::TestRequest::default().to_http_request();
        req.extensions_mut().insert(AuthContext {
            scopes: vec!["read:telemetry".to_string(), "write:config".to_string()],
            user_id: None,
            session_id: None,
            service_key_label: Some("domain11-validation".to_string()),
        });
        let app_state = web::Data::new(crate::api::test_support::test_app_state());

        let result = require_device_owner(&req, &app_state, "device_001").await;

        assert_eq!(
            result.expect("service principal should pass device guard"),
            0
        );
    }
}
