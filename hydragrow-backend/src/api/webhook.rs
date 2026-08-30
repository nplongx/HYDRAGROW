use actix_web::http::StatusCode;
use actix_web::{HttpResponse, Responder, web};
use serde_json::{Value, json};

use crate::AppState;
use crate::models::script::ActionCommandOutput;
use crate::services::action_dispatch::ActionDispatchError;
use crate::services::action_dispatch::dispatch_action_command;

/// Webhook cho hệ thống ngoài (Zapier, Home Assistant, v.v.) gửi lệnh điều khiển
/// trực tiếp. Dùng LẠI đúng pipeline `action_dispatch::dispatch_action_command` mà
/// action_command script (Phase 1) dùng — cùng safety gate, cùng wire format MQTT
/// (`MqttCommandOut`), không có đường tắt nào bỏ qua safety check.
///
/// Auth: header `X-API-Key` (đã bọc sẵn bởi `auth_middleware` cho toàn bộ scope
/// `/devices/{device_id}` — không cần khai báo gì thêm ở đây).
///
/// Ví dụ gọi từ Zapier/Home Assistant:
/// ```text
/// POST /devices/dev1/webhook/action
/// X-API-Key: <api_key>
/// Content-Type: application/json
///
/// { "action": "dose", "pump": "PH_DOWN", "dose_ml": 3, "pwm": 80 }
/// ```
async fn receive_webhook_action(
    path: web::Path<String>,
    body: web::Json<ActionCommandOutput>,
    app_state: web::Data<AppState>,
) -> impl Responder {
    let device_id = path.into_inner();
    let output = body.into_inner();

    let safety_config =
        match crate::db::postgres::get_safety_config(&app_state.pg_pool, &device_id).await {
            Ok(cfg) => cfg,
            Err(e) => {
                tracing::error!(error = ?e, device_id, "Lỗi đọc safety_config cho webhook action");
                return HttpResponse::InternalServerError().json(
                    json!({ "error": "Database Error", "message": "Không đọc được safety_config" }),
                );
            }
        };
    let calibration = crate::db::postgres::fetch_dosing_calibration(&app_state.pg_pool, &device_id)
        .await
        .unwrap_or(None);
    let limits = hydragrow_shared::safety::DoseSafetyLimits {
        max_dose_per_cycle_ml: safety_config.max_dose_per_cycle,
        max_dose_per_hour_ml: safety_config.max_dose_per_hour,
        cooldown_sec: safety_config.cooldown_sec as u64,
    };
    let now_sec = chrono::Utc::now().timestamp() as u64;

    // hourly_history_ml=&[], last_dose_at_sec=None: cùng giới hạn đã ghi nhận từ
    // Phase 1 (sensors.rs) — chưa có nguồn theo dõi lịch sử liều thật cho MỌI
    // đường gọi action_command, không phải thiếu sót riêng của webhook.
    match dispatch_action_command(
        &app_state,
        &device_id,
        output,
        &limits,
        &[],
        now_sec,
        None,
        calibration.as_ref(),
    )
    .await
    {
        Ok(()) => HttpResponse::Ok().json(json!({ "status": "success" })),
        Err(e) => {
            if matches!(
                e,
                crate::services::action_dispatch::ActionDispatchError::Mqtt(_)
            ) {
                tracing::error!(error = ?e, device_id, "Lỗi publish MQTT cho webhook action");
            }
            let (status, body) = dispatch_error_to_response(&e);
            HttpResponse::build(status).json(body)
        }
    }
}

pub fn init_routes(cfg: &mut web::ServiceConfig) {
    cfg.route("/webhook/action", web::post().to(receive_webhook_action));
}

fn dispatch_error_to_response(err: &ActionDispatchError) -> (StatusCode, Value) {
    match err {
        ActionDispatchError::Safety(violation) => (
            StatusCode::UNPROCESSABLE_ENTITY,
            json!({ "error": "Safety Check Failed", "message": format!("{violation:?}") }),
        ),
        ActionDispatchError::UnknownPump(msg) => (
            StatusCode::BAD_REQUEST,
            json!({ "error": "Bad Request", "message": msg }),
        ),
        ActionDispatchError::UnknownAction(action) => (
            StatusCode::BAD_REQUEST,
            json!({ "error": "Bad Request", "message": format!("action không hợp lệ: {action}") }),
        ),
        ActionDispatchError::Mqtt(_) => (
            StatusCode::INTERNAL_SERVER_ERROR,
            json!({ "error": "MQTT Error", "message": "Không publish được lệnh" }),
        ),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::services::action_dispatch::ActionDispatchError;
    use actix_web::http::StatusCode;

    #[test]
    fn safety_violation_maps_to_422() {
        let err = ActionDispatchError::Safety(
            hydragrow_shared::safety::DoseSafetyViolation::CooldownActive {
                seconds_remaining: 10,
            },
        );
        let (status, _) = dispatch_error_to_response(&err);
        assert_eq!(status, StatusCode::UNPROCESSABLE_ENTITY);
    }

    #[test]
    fn unknown_pump_maps_to_400() {
        let err = ActionDispatchError::UnknownPump("X".to_string());
        let (status, _) = dispatch_error_to_response(&err);
        assert_eq!(status, StatusCode::BAD_REQUEST);
    }

    #[test]
    fn unknown_action_maps_to_400() {
        let err = ActionDispatchError::UnknownAction("foo".to_string());
        let (status, _) = dispatch_error_to_response(&err);
        assert_eq!(status, StatusCode::BAD_REQUEST);
    }

    #[test]
    fn mqtt_error_maps_to_500() {
        let err = ActionDispatchError::Mqtt(anyhow::anyhow!("boom"));
        let (status, _) = dispatch_error_to_response(&err);
        assert_eq!(status, StatusCode::INTERNAL_SERVER_ERROR);
    }
}
