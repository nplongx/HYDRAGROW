use actix_web::{HttpMessage, HttpRequest, HttpResponse, Responder, web};
use serde::{Deserialize, Serialize};
use chrono::Utc;
use tracing::{info, warn};
use sqlx::Row;

use crate::AppState;
use crate::api::middleware::auth::AuthContext;
use crate::api::mqtt_utils::publish_command;
use hydragrow_shared::{MqttCommandOut, MqttCommandParams};

#[derive(Debug, Serialize, Deserialize)]
pub struct DeviceBackup {
    pub schema_version: u8,
    pub exported_at: String,
    pub device_id: String,
    pub device_config: serde_json::Value,
    pub recipe: Option<serde_json::Value>,
}

fn auth_from(req: &HttpRequest) -> AuthContext {
    req.extensions().get::<AuthContext>().cloned().unwrap_or_default()
}

/// GET /device/{id}/admin/backup — Export config + recipe hiện tại.
/// Backend lấy từ DB (device_config table) và trả JSON để user download.
pub async fn export_backup(
    path: web::Path<String>,
    req: HttpRequest,
    app_state: web::Data<AppState>,
) -> impl Responder {
    let device_id = path.into_inner();
    let auth = auth_from(&req);
    if !auth.has_scope("device:admin") && !auth.has_scope("write:config") {
        return HttpResponse::Forbidden()
            .json(serde_json::json!({"error": "Missing scope: device:admin or write:config"}));
    }

    // Lấy config từ DB — sử dụng query json
    let config_row = sqlx::query(
        "SELECT row_to_json(device_config) as config_json FROM device_config WHERE device_id = $1 ORDER BY last_updated DESC LIMIT 1"
    )
    .bind(&device_id)
    .fetch_optional(&app_state.pg_pool)
    .await;

    let device_config = match config_row {
        Ok(Some(row)) => row.try_get::<serde_json::Value, _>("config_json").unwrap_or(serde_json::Value::Null),
        _ => serde_json::Value::Null,
    };

    // Lấy active recipe từ DB
    let recipe_row = sqlx::query(
        "SELECT row_to_json(crop_recipes) as recipe_json FROM device_active_recipes JOIN crop_recipes ON device_active_recipes.recipe_id = crop_recipes.id WHERE device_id = $1 LIMIT 1"
    )
    .bind(&device_id)
    .fetch_optional(&app_state.pg_pool)
    .await;

    let recipe = match recipe_row {
        Ok(Some(row)) => row.try_get::<serde_json::Value, _>("recipe_json").ok(),
        _ => None,
    };

    let backup = DeviceBackup {
        schema_version: 1,
        exported_at: Utc::now().to_rfc3339(),
        device_id: device_id.clone(),
        device_config,
        recipe,
    };

    info!(%device_id, "Exported device backup");
    HttpResponse::Ok()
        .content_type("application/json")
        .insert_header((
            "Content-Disposition",
            format!("attachment; filename=\"hydragrow_backup_{}.json\"", device_id),
        ))
        .json(backup)
}

/// POST /device/{id}/admin/restore — Import backup JSON, push config + recipe xuống device.
pub async fn import_backup(
    path: web::Path<String>,
    req: HttpRequest,
    body: web::Json<DeviceBackup>,
    app_state: web::Data<AppState>,
) -> impl Responder {
    let device_id = path.into_inner();
    let auth = auth_from(&req);
    if !auth.has_scope("device:admin") {
        return HttpResponse::Forbidden()
            .json(serde_json::json!({"error": "Missing scope: device:admin"}));
    }

    if body.schema_version != 1 {
        return HttpResponse::BadRequest()
            .json(serde_json::json!({"error": "Unsupported backup schema_version"}));
    }

    info!(%device_id, exported_at = %body.exported_at, "Importing device backup");

    // Push config xuống device qua MQTT (action: set_config)
    let config_cmd = MqttCommandOut {
        target: "all".to_string(),
        action: "set_config".to_string(),
        params: Some(MqttCommandParams {
            pump_id: None, duration_sec: None, pwm: None, state: None,
            ota_url: Some(serde_json::to_string(&body.device_config).unwrap_or_default()),
            candidates: None,
        }),
        ts: None, nonce: None, signature: None,
    };
    if let Err(e) = publish_command(&app_state, &device_id, &config_cmd).await {
        warn!(%device_id, ?e, "Failed to push config via MQTT");
    }

    // Push recipe nếu có
    if let Some(recipe) = &body.recipe {
        let recipe_cmd = MqttCommandOut {
            target: "all".to_string(),
            action: "set_recipe".to_string(),
            params: Some(MqttCommandParams {
                pump_id: None, duration_sec: None, pwm: None, state: None,
                ota_url: Some(serde_json::to_string(recipe).unwrap_or_default()),
                candidates: None,
            }),
            ts: None, nonce: None, signature: None,
        };
        if let Err(e) = publish_command(&app_state, &device_id, &recipe_cmd).await {
            warn!(%device_id, ?e, "Failed to push recipe via MQTT");
        }
    }

    HttpResponse::Accepted()
        .json(serde_json::json!({"status": "restore_triggered", "device_id": device_id}))
}

pub fn init_routes(cfg: &mut web::ServiceConfig) {
    cfg.route("/backup", web::get().to(export_backup))
       .route("/restore", web::post().to(import_backup));
}
