use actix_web::{HttpMessage, HttpRequest, HttpResponse, Responder, web};
use chrono::{DateTime, Utc};
use hydragrow_shared::ControllerConfig;
use serde_json::json;
use tracing::{error, info, instrument};

use crate::AppState;
use crate::api::middleware::auth::AuthContext;
use crate::db::postgres::{NewSystemEventRecord, SystemEventRecord, insert_system_event};
use crate::models::config::{
    DeviceConfig, DosingCalibration, SafetyConfig, SensorCalibration, WaterConfig, from_db_rows,
};

fn require_write_config_scope(req: &HttpRequest) -> Result<(), HttpResponse> {
    let auth = req
        .extensions()
        .get::<AuthContext>()
        .cloned()
        .unwrap_or_default();

    if auth.has_scope("write:config") {
        Ok(())
    } else {
        Err(HttpResponse::Forbidden().json(json!({
            "error": "Missing required scope",
            "required_scope": "write:config"
        })))
    }
}

fn require_read_telemetry_scope(req: &HttpRequest) -> Result<(), HttpResponse> {
    let auth = req
        .extensions()
        .get::<AuthContext>()
        .cloned()
        .unwrap_or_default();
    if auth.has_scope("read:telemetry") {
        Ok(())
    } else {
        Err(HttpResponse::Forbidden().json(json!({
            "error": "Missing required scope",
            "required_scope": "read:telemetry"
        })))
    }
}

#[derive(serde::Deserialize, serde::Serialize)]
pub struct UnifiedConfigRequest {
    pub device_config: DeviceConfig,
    pub water_config: WaterConfig,
    pub safety_config: SafetyConfig,
    pub sensor_calibration: SensorCalibration,
    pub dosing_calibration: DosingCalibration,
}

#[derive(serde::Deserialize)]
pub struct FinishCalibrationRequest {
    pub mode: String,
    pub sample_points: Vec<f32>,
    pub ph_v7: f32,
    pub ph_v4: f32,
    pub ph_v10: Option<f32>,
    pub error: f32,
    pub finished_at: Option<DateTime<Utc>>,
}

// ==========================================
// HELPER FUNCTIONS
// ==========================================

async fn fetch_unified_config_concurrently(
    pool: &sqlx::PgPool,
    device_id: &str,
) -> Result<ControllerConfig, String> {
    let (dev_res, water_res, safe_res, dose_res, sens_res) = tokio::join!(
        sqlx::query_as::<_, DeviceConfig>("SELECT * FROM device_config WHERE device_id = $1")
            .bind(device_id)
            .fetch_optional(pool),
        sqlx::query_as::<_, WaterConfig>("SELECT * FROM water_config WHERE device_id = $1")
            .bind(device_id)
            .fetch_optional(pool),
        sqlx::query_as::<_, SafetyConfig>("SELECT * FROM safety_config WHERE device_id = $1")
            .bind(device_id)
            .fetch_optional(pool),
        sqlx::query_as::<_, DosingCalibration>(
            "SELECT * FROM dosing_calibration WHERE device_id = $1"
        )
        .bind(device_id)
        .fetch_optional(pool),
        sqlx::query_as::<_, SensorCalibration>(
            "SELECT * FROM sensor_calibration WHERE device_id = $1"
        )
        .bind(device_id)
        .fetch_optional(pool)
    );

    let dev = dev_res
        .map_err(|e| format!("DB Error dev: {}", e))?
        .ok_or_else(|| "Device base config not found".to_string())?;

    let water = water_res
        .map_err(|e| format!("DB Error water: {}", e))?
        .unwrap_or_else(|| WaterConfig {
            device_id: device_id.to_string(),
            ..Default::default()
        });

    let safe = safe_res
        .map_err(|e| format!("DB Error safety: {}", e))?
        .unwrap_or_else(|| SafetyConfig {
            device_id: device_id.to_string(),
            ..Default::default()
        });

    let dose = dose_res
        .map_err(|e| format!("DB Error dosing: {}", e))?
        .unwrap_or_else(|| DosingCalibration {
            device_id: device_id.to_string(),
            ..Default::default()
        });

    let sens = sens_res
        .map_err(|e| format!("DB Error sensor: {}", e))?
        .unwrap_or_else(|| SensorCalibration {
            device_id: device_id.to_string(),
            ph_v7: 2.5,
            ph_v4: 3.04,
            ph_v10: None,
            ph_calibration_mode: "2-point".into(),
            ec_factor: 880.0,
            ec_offset: 0.0,
            temp_offset: 0.0,
            temp_compensation_beta: 0.02,
            publish_interval: 5000,
            moving_average_window: 10,
            enable_ph_sensor: true,
            enable_ec_sensor: true,
            enable_temp_sensor: true,
            enable_water_level_sensor: true,
            last_calibrated: Utc::now(),
        });

    Ok(from_db_rows(&dev, &water, &safe, &dose, &sens))
}

pub async fn sync_config_to_esp32(
    app_state: &web::Data<AppState>,
    device_id: &str,
) -> Result<(), String> {
    // Persist desired state. ConfigurationSync owns MQTT delivery so offline
    // devices converge after reconnect instead of losing an API update.
    let payload = fetch_unified_config_concurrently(&app_state.pg_pool, device_id).await?;
    let sens = sqlx::query_as::<_, SensorCalibration>(
        "SELECT * FROM sensor_calibration WHERE device_id = $1",
    )
    .bind(device_id)
    .fetch_optional(&app_state.pg_pool)
    .await
    .map_err(|e| format!("Lỗi đọc Sensor Config: {:?}", e))?;

    let sensor_payload = if let Some(sensor_config) = sens {
        json!({
            "ph_v7":                   sensor_config.ph_v7,
            "ph_v4":                   sensor_config.ph_v4,
            "ph_v10":                  sensor_config.ph_v10,
            "ph_calibration_mode":     sensor_config.ph_calibration_mode,
            "ec_factor":               sensor_config.ec_factor,
            "ec_offset":               sensor_config.ec_offset,
            "temp_offset":             sensor_config.temp_offset,
            "temp_compensation_beta":  sensor_config.temp_compensation_beta,
            "moving_average_window":   sensor_config.moving_average_window,
            "publish_interval":        sensor_config.publish_interval,
            "enable_ph_sensor":        sensor_config.enable_ph_sensor,
            "enable_ec_sensor":        sensor_config.enable_ec_sensor,
            "enable_temp_sensor":      sensor_config.enable_temp_sensor,
            "enable_water_level_sensor": sensor_config.enable_water_level_sensor,
        })
    } else {
        json!({})
    };

    let controller_value = serde_json::to_value(&payload)
        .map_err(|e| format!("Lỗi serialize controller config: {e:?}"))?;
    let version = crate::services::configuration::persist_desired_revision(
        &app_state.pg_pool,
        device_id,
        controller_value,
        sensor_payload,
    )
    .await?;

    info!(
        device_id,
        config_version = version,
        "ConfigurationSync desired revision persisted"
    );
    Ok(())
}

async fn upsert_water_db(
    executor: impl sqlx::Executor<'_, Database = sqlx::Postgres>,
    config: &WaterConfig,
    now: &chrono::DateTime<chrono::Utc>,
) -> Result<(), sqlx::Error> {
    sqlx::query(
        r#"
        INSERT INTO water_config (
            device_id, tank_height, water_level_min, water_level_target, water_level_max,
            water_level_drain, water_level_tolerance, auto_refill_enabled,
            auto_drain_overflow, auto_dilute_enabled, dilute_drain_amount_cm,
            scheduled_water_change_enabled, water_change_cron, scheduled_drain_amount_cm,
            misting_on_duration_ms, misting_off_duration_ms,
            misting_temp_threshold, high_temp_misting_on_duration_ms, high_temp_misting_off_duration_ms,
            last_updated
        ) VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9, $10, $11, $12, $13, $14, $15, $16, $17, $18, $19, $20)
        ON CONFLICT(device_id) DO UPDATE SET
            tank_height = EXCLUDED.tank_height,
            water_level_min = EXCLUDED.water_level_min,
            water_level_target = EXCLUDED.water_level_target,
            water_level_max = EXCLUDED.water_level_max,
            water_level_drain = EXCLUDED.water_level_drain,
            water_level_tolerance = EXCLUDED.water_level_tolerance,
            auto_refill_enabled = EXCLUDED.auto_refill_enabled,
            auto_drain_overflow = EXCLUDED.auto_drain_overflow,
            auto_dilute_enabled = EXCLUDED.auto_dilute_enabled,
            dilute_drain_amount_cm = EXCLUDED.dilute_drain_amount_cm,
            scheduled_water_change_enabled = EXCLUDED.scheduled_water_change_enabled,
            water_change_cron = EXCLUDED.water_change_cron,
            scheduled_drain_amount_cm = EXCLUDED.scheduled_drain_amount_cm,
            misting_on_duration_ms = EXCLUDED.misting_on_duration_ms,
            misting_off_duration_ms = EXCLUDED.misting_off_duration_ms,
            misting_temp_threshold = EXCLUDED.misting_temp_threshold,
            high_temp_misting_on_duration_ms = EXCLUDED.high_temp_misting_on_duration_ms,
            high_temp_misting_off_duration_ms = EXCLUDED.high_temp_misting_off_duration_ms,
            last_updated = EXCLUDED.last_updated
        "#,
    )
    .bind(&config.device_id)
    .bind(config.tank_height)
    .bind(config.water_level_min)
    .bind(config.water_level_target)
    .bind(config.water_level_max)
    .bind(config.water_level_drain)
    .bind(config.water_level_tolerance)
    .bind(config.auto_refill_enabled)
    .bind(config.auto_drain_overflow)
    .bind(config.auto_dilute_enabled)
    .bind(config.dilute_drain_amount_cm)
    .bind(config.scheduled_water_change_enabled)
    .bind(&config.water_change_cron)
    .bind(config.scheduled_drain_amount_cm)
    .bind(config.misting_on_duration_ms)
    .bind(config.misting_off_duration_ms)
    // Thêm 3 bind cho cấu hình nóng
    .bind(config.misting_temp_threshold)
    .bind(config.high_temp_misting_on_duration_ms)
    .bind(config.high_temp_misting_off_duration_ms)
    .bind(now)
    .execute(executor)
    .await?;
    Ok(())
}

async fn upsert_sensor_db(
    executor: impl sqlx::Executor<'_, Database = sqlx::Postgres>,
    cal: &SensorCalibration,
    now: &chrono::DateTime<chrono::Utc>,
) -> Result<(), sqlx::Error> {
    sqlx::query(
        r#"
        INSERT INTO sensor_calibration (
            device_id, ph_v7, ph_v4, ph_v10, ph_calibration_mode, ec_factor, ec_offset, temp_offset,
            temp_compensation_beta, publish_interval, moving_average_window,
            enable_ph_sensor, enable_ec_sensor, enable_temp_sensor, enable_water_level_sensor, last_calibrated
        ) VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9, $10, $11, $12, $13, $14, $15, $16)
        ON CONFLICT(device_id) DO UPDATE SET
            ph_v7 = EXCLUDED.ph_v7, ph_v4 = EXCLUDED.ph_v4, ph_v10 = EXCLUDED.ph_v10,
            ph_calibration_mode = EXCLUDED.ph_calibration_mode, ec_factor = EXCLUDED.ec_factor,
            ec_offset = EXCLUDED.ec_offset, temp_offset = EXCLUDED.temp_offset,
            temp_compensation_beta = EXCLUDED.temp_compensation_beta,
            publish_interval = EXCLUDED.publish_interval, moving_average_window = EXCLUDED.moving_average_window,
            enable_ph_sensor = EXCLUDED.enable_ph_sensor, enable_ec_sensor = EXCLUDED.enable_ec_sensor,
            enable_temp_sensor = EXCLUDED.enable_temp_sensor, enable_water_level_sensor = EXCLUDED.enable_water_level_sensor,
            last_calibrated = EXCLUDED.last_calibrated
        "#
    )
    .bind(&cal.device_id)
    .bind(cal.ph_v7)
    .bind(cal.ph_v4)
    .bind(cal.ph_v10)
    .bind(&cal.ph_calibration_mode)
    .bind(cal.ec_factor)
    .bind(cal.ec_offset)
    .bind(cal.temp_offset)
    .bind(cal.temp_compensation_beta)
    .bind(cal.publish_interval)
    .bind(cal.moving_average_window)
    .bind(cal.enable_ph_sensor)
    .bind(cal.enable_ec_sensor)
    .bind(cal.enable_temp_sensor)
    .bind(cal.enable_water_level_sensor)
    .bind(now)
    .execute(executor).await?;
    Ok(())
}

fn default_sensor_calibration(device_id: &str, now: DateTime<Utc>) -> SensorCalibration {
    SensorCalibration {
        device_id: device_id.to_string(),
        ph_v7: 2.5,
        ph_v4: 3.04,
        ph_v10: None,
        ph_calibration_mode: "2-point".into(),
        ec_factor: 880.0,
        ec_offset: 0.0,
        temp_offset: 0.0,
        temp_compensation_beta: 0.02,
        publish_interval: 5000,
        moving_average_window: 10,
        enable_ph_sensor: true,
        enable_ec_sensor: true,
        enable_temp_sensor: true,
        enable_water_level_sensor: true,
        last_calibrated: now,
    }
}

fn validate_dosing_constraints(dose: &DosingCalibration) -> Result<(), String> {
    if !(1..=100).contains(&dose.dosing_pwm_percent) {
        return Err("dosing_pwm_percent must be in range [1..100]".to_string());
    }

    if !(0..=100).contains(&dose.dosing_min_pwm_percent) {
        return Err("dosing_min_pwm_percent must be in range [0..100]".to_string());
    }

    if dose.pump_a_capacity_ml_per_sec <= 0.0 {
        return Err("pump_a_capacity_ml_per_sec must be > 0".to_string());
    }
    if dose.pump_b_capacity_ml_per_sec <= 0.0 {
        return Err("pump_b_capacity_ml_per_sec must be > 0".to_string());
    }
    if dose.pump_ph_up_capacity_ml_per_sec <= 0.0 {
        return Err("pump_ph_up_capacity_ml_per_sec must be > 0".to_string());
    }
    if dose.pump_ph_down_capacity_ml_per_sec <= 0.0 {
        return Err("pump_ph_down_capacity_ml_per_sec must be > 0".to_string());
    }
    if dose.dosing_min_pwm_percent > dose.dosing_pwm_percent {
        return Err("dosing_min_pwm_percent must be <= dosing_pwm_percent".to_string());
    }

    Ok(())
}

async fn upsert_dosing_db(
    executor: impl sqlx::Executor<'_, Database = sqlx::Postgres>,
    cal: &DosingCalibration,
    now: &chrono::DateTime<chrono::Utc>,
) -> Result<(), sqlx::Error> {
    sqlx::query(
        r#"
        INSERT INTO dosing_calibration (
            device_id, ec_gain_per_ml, ph_shift_up_per_ml,
            ph_shift_down_per_ml, active_mixing_sec, sensor_stabilize_sec, ec_step_ratio, ph_step_ratio,
            pump_a_capacity_ml_per_sec, pump_b_capacity_ml_per_sec,
            pump_ph_up_capacity_ml_per_sec, pump_ph_down_capacity_ml_per_sec,
            soft_start_duration, last_calibrated,
            scheduled_mixing_interval_sec, scheduled_mixing_duration_sec,
            dosing_pwm_percent, osaka_mixing_pwm_percent, osaka_misting_pwm_percent,
            dosing_min_pwm_percent, pump_a_min_pwm_percent, pump_b_min_pwm_percent,
            pump_ph_up_min_pwm_percent, pump_ph_down_min_pwm_percent, dosing_pulse_on_ms,
            dosing_pulse_off_ms, dosing_min_dose_ml, dosing_max_pulse_count_per_cycle
        ) VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9, $10, $11, $12, $13, $14, $15, $16, $17, $18, $19, $20, $21, $22, $23, $24, $25, $26, $27, $28)
        ON CONFLICT(device_id) DO UPDATE SET
            ec_gain_per_ml = EXCLUDED.ec_gain_per_ml,
            ph_shift_up_per_ml = EXCLUDED.ph_shift_up_per_ml, ph_shift_down_per_ml = EXCLUDED.ph_shift_down_per_ml,
            active_mixing_sec = EXCLUDED.active_mixing_sec, sensor_stabilize_sec = EXCLUDED.sensor_stabilize_sec,
            ec_step_ratio = EXCLUDED.ec_step_ratio, ph_step_ratio = EXCLUDED.ph_step_ratio,
            pump_a_capacity_ml_per_sec = EXCLUDED.pump_a_capacity_ml_per_sec,
            pump_b_capacity_ml_per_sec = EXCLUDED.pump_b_capacity_ml_per_sec,
            pump_ph_up_capacity_ml_per_sec = EXCLUDED.pump_ph_up_capacity_ml_per_sec,
            pump_ph_down_capacity_ml_per_sec = EXCLUDED.pump_ph_down_capacity_ml_per_sec,
            soft_start_duration = EXCLUDED.soft_start_duration, scheduled_mixing_interval_sec = EXCLUDED.scheduled_mixing_interval_sec,
            scheduled_mixing_duration_sec = EXCLUDED.scheduled_mixing_duration_sec, dosing_pwm_percent = EXCLUDED.dosing_pwm_percent,
            osaka_mixing_pwm_percent = EXCLUDED.osaka_mixing_pwm_percent, osaka_misting_pwm_percent = EXCLUDED.osaka_misting_pwm_percent,
            dosing_min_pwm_percent = EXCLUDED.dosing_min_pwm_percent,
            pump_a_min_pwm_percent = EXCLUDED.pump_a_min_pwm_percent,
            pump_b_min_pwm_percent = EXCLUDED.pump_b_min_pwm_percent,
            pump_ph_up_min_pwm_percent = EXCLUDED.pump_ph_up_min_pwm_percent,
            pump_ph_down_min_pwm_percent = EXCLUDED.pump_ph_down_min_pwm_percent,
            dosing_pulse_on_ms = EXCLUDED.dosing_pulse_on_ms,
            dosing_pulse_off_ms = EXCLUDED.dosing_pulse_off_ms,
            dosing_min_dose_ml = EXCLUDED.dosing_min_dose_ml,
            dosing_max_pulse_count_per_cycle = EXCLUDED.dosing_max_pulse_count_per_cycle,
            last_calibrated = EXCLUDED.last_calibrated
        "#
    )
    .bind(&cal.device_id)
    .bind(cal.ec_gain_per_ml)
    .bind(cal.ph_shift_up_per_ml)
    .bind(cal.ph_shift_down_per_ml)
    .bind(cal.active_mixing_sec)
    .bind(cal.sensor_stabilize_sec)
    .bind(cal.ec_step_ratio)
    .bind(cal.ph_step_ratio)
    .bind(cal.pump_a_capacity_ml_per_sec)
    .bind(cal.pump_b_capacity_ml_per_sec)
    .bind(cal.pump_ph_up_capacity_ml_per_sec)
    .bind(cal.pump_ph_down_capacity_ml_per_sec)
    .bind(cal.soft_start_duration)
    .bind(now)
    .bind(cal.scheduled_mixing_interval_sec)
    .bind(cal.scheduled_mixing_duration_sec)
    .bind(cal.dosing_pwm_percent)
    .bind(cal.osaka_mixing_pwm_percent)
    .bind(cal.osaka_misting_pwm_percent)
    .bind(cal.dosing_min_pwm_percent)
    .bind(cal.pump_a_min_pwm_percent)
    .bind(cal.pump_b_min_pwm_percent)
    .bind(cal.pump_ph_up_min_pwm_percent)
    .bind(cal.pump_ph_down_min_pwm_percent)
    .bind(cal.dosing_pulse_on_ms)
    .bind(cal.dosing_pulse_off_ms)
    .bind(cal.dosing_min_dose_ml)
    .bind(cal.dosing_max_pulse_count_per_cycle)
    .execute(executor).await?;

    Ok(())
}

#[instrument(skip(app_state, req))]
pub async fn update_unified_config(
    path: web::Path<String>,
    req: web::Json<UnifiedConfigRequest>,
    app_state: web::Data<AppState>,
    http_req: HttpRequest,
) -> impl Responder {
    if let Err(resp) = require_write_config_scope(&http_req) {
        return resp;
    }
    let device_id = path.into_inner();
    if let Err(resp) =
        crate::api::device_pairing::require_device_owner(&http_req, &app_state, &device_id).await
    {
        return resp;
    }
    let mut payload = req.into_inner();
    let now = Utc::now();

    payload.device_config.device_id = device_id.clone();
    payload.device_config.last_updated = now;
    payload.safety_config.device_id = device_id.clone();
    payload.safety_config.last_updated = now;
    payload.water_config.device_id = device_id.clone();
    payload.sensor_calibration.device_id = device_id.clone();
    payload.dosing_calibration.device_id = device_id.clone();
    if let Err(msg) = validate_dosing_constraints(&payload.dosing_calibration) {
        return HttpResponse::BadRequest().json(json!({"error": msg}));
    }

    let mut tx = match app_state.pg_pool.begin().await {
        Ok(tx) => tx,
        Err(e) => {
            error!("Failed to begin config transaction: {:?}", e);
            return HttpResponse::InternalServerError().json(json!({"error": "DB Error"}));
        }
    };
    if let Err(e) =
        crate::db::postgres::upsert_device_config(&mut *tx, &payload.device_config).await
    {
        error!("Failed to update device config: {:?}", e);
        return HttpResponse::InternalServerError().json(json!({"error": "DB Error: Device"}));
    }
    if let Err(e) =
        crate::db::postgres::upsert_safety_config(&mut *tx, &payload.safety_config).await
    {
        error!("Failed to update safety config: {:?}", e);
        return HttpResponse::InternalServerError().json(json!({"error": "DB Error: Safety"}));
    }
    if let Err(e) = upsert_water_db(&mut *tx, &payload.water_config, &now).await {
        error!("Failed to update water config: {:?}", e);
        return HttpResponse::InternalServerError().json(json!({"error": "DB Error: Water"}));
    }
    if let Err(e) = upsert_sensor_db(&mut *tx, &payload.sensor_calibration, &now).await {
        error!("Failed to update sensor config: {:?}", e);
        return HttpResponse::InternalServerError().json(json!({"error": "DB Error: Sensor"}));
    }
    if let Err(e) = upsert_dosing_db(&mut *tx, &payload.dosing_calibration, &now).await {
        error!("Failed to update dosing config: {:?}", e);
        return HttpResponse::InternalServerError().json(json!({"error": "DB Error: Dosing"}));
    }
    if let Err(e) = tx.commit().await {
        error!("Failed to commit unified config transaction: {:?}", e);
        return HttpResponse::InternalServerError().json(json!({"error": "DB Error"}));
    }

    let audit_event = NewSystemEventRecord {
        device_id: device_id.clone(),
        level: "info".to_string(),
        category: "user_action".to_string(),
        title: "Cập nhật cấu hình trạm".to_string(),
        message: format!("Người dùng đã cập nhật cấu hình cho trạm {device_id}."),
        reason: Some("config_update".to_string()),
        metadata: Some(serde_json::json!({
            "event_type": "config_change",
            "scope": "unified",
            "ec_target": payload.device_config.ec_target,
            "ph_target": payload.device_config.ph_target,
            "control_mode": payload.device_config.control_mode,
            "is_enabled": payload.device_config.is_enabled,
        })),
        timestamp: now.timestamp_millis(),
        source: "rule".to_string(),
        primary_reason_code: None,
    };
    let _ = insert_system_event(&app_state.pg_pool, &audit_event).await;

    if let Err(e) = sync_config_to_esp32(&app_state, &device_id).await {
        error!("Lưu DB thành công nhưng lỗi MQTT: {}", e);
        return HttpResponse::Accepted().json(json!({
            "status": "partial_success",
            "message": "Đã lưu CSDL nhưng không thể đồng bộ tới thiết bị do mất kết nối mạng."
        }));
    }

    HttpResponse::Ok().json(json!({"status": "success"}))
}

#[instrument(skip(app_state))]
pub async fn get_unified_device_config(
    path: web::Path<String>,
    app_state: web::Data<AppState>,
    http_req: HttpRequest,
) -> impl Responder {
    if let Err(resp) = require_read_telemetry_scope(&http_req) {
        return resp;
    }
    let device_id = path.into_inner();
    if let Err(resp) =
        crate::api::device_pairing::require_device_owner(&http_req, &app_state, &device_id).await
    {
        return resp;
    }
    let pool = &app_state.pg_pool;

    let (dev_res, water_res, safe_res, dose_res, sens_res) = tokio::join!(
        sqlx::query_as::<_, DeviceConfig>("SELECT * FROM device_config WHERE device_id = $1")
            .bind(&device_id)
            .fetch_optional(pool),
        sqlx::query_as::<_, WaterConfig>("SELECT * FROM water_config WHERE device_id = $1")
            .bind(&device_id)
            .fetch_optional(pool),
        sqlx::query_as::<_, SafetyConfig>("SELECT * FROM safety_config WHERE device_id = $1")
            .bind(&device_id)
            .fetch_optional(pool),
        sqlx::query_as::<_, DosingCalibration>(
            "SELECT * FROM dosing_calibration WHERE device_id = $1"
        )
        .bind(&device_id)
        .fetch_optional(pool),
        sqlx::query_as::<_, SensorCalibration>(
            "SELECT * FROM sensor_calibration WHERE device_id = $1"
        )
        .bind(&device_id)
        .fetch_optional(pool)
    );

    let device_config = match dev_res {
        Ok(Some(config)) => config,
        Ok(None) => {
            return HttpResponse::NotFound().json(json!({
                "error": "Configuration not found",
                "reason": "device_config_missing"
            }));
        }
        Err(e) => {
            error!(device_id = %device_id, error = ?e, "Failed to read device config");
            return HttpResponse::InternalServerError().json(json!({
                "error": "Failed to read configuration"
            }));
        }
    };

    let water_config = match water_res {
        Ok(Some(config)) => config,
        Ok(None) => WaterConfig {
            device_id: device_id.clone(),
            ..Default::default()
        },
        Err(e) => {
            error!(device_id = %device_id, error = ?e, "Failed to read water config");
            return HttpResponse::InternalServerError()
                .json(json!({"error": "Failed to read configuration"}));
        }
    };
    let safety_config = match safe_res {
        Ok(Some(config)) => config,
        Ok(None) => SafetyConfig {
            device_id: device_id.clone(),
            ..Default::default()
        },
        Err(e) => {
            error!(device_id = %device_id, error = ?e, "Failed to read safety config");
            return HttpResponse::InternalServerError()
                .json(json!({"error": "Failed to read configuration"}));
        }
    };
    let dosing_calibration = match dose_res {
        Ok(Some(config)) => config,
        Ok(None) => DosingCalibration {
            device_id: device_id.clone(),
            ..Default::default()
        },
        Err(e) => {
            error!(device_id = %device_id, error = ?e, "Failed to read dosing calibration");
            return HttpResponse::InternalServerError()
                .json(json!({"error": "Failed to read configuration"}));
        }
    };
    let sensor_calibration = match sens_res {
        Ok(Some(config)) => config,
        Ok(None) => default_sensor_calibration(&device_id, Utc::now()),
        Err(e) => {
            error!(device_id = %device_id, error = ?e, "Failed to read sensor calibration");
            return HttpResponse::InternalServerError()
                .json(json!({"error": "Failed to read configuration"}));
        }
    };

    let response_payload = UnifiedConfigRequest {
        device_config,
        water_config,
        safety_config,
        dosing_calibration,
        sensor_calibration,
    };

    HttpResponse::Ok().json(response_payload)
}

#[instrument(skip(app_state))]
pub async fn get_config(
    path: web::Path<String>,
    app_state: web::Data<AppState>,
    http_req: HttpRequest,
) -> impl Responder {
    if let Err(resp) = require_read_telemetry_scope(&http_req) {
        return resp;
    }
    let device_id = path.into_inner();
    if let Err(resp) =
        crate::api::device_pairing::require_device_owner(&http_req, &app_state, &device_id).await
    {
        return resp;
    }
    match crate::db::postgres::get_device_config(&app_state.pg_pool, &device_id).await {
        Ok(config) => HttpResponse::Ok().json(config),
        Err(e) => {
            tracing::warn!("Config not found or DB error: {:?}", e);
            HttpResponse::NotFound().json(json!({"error": "Configuration not found"}))
        }
    }
}

#[instrument(skip(app_state, payload))]
pub async fn update_config(
    path: web::Path<String>,
    payload: web::Json<DeviceConfig>,
    app_state: web::Data<AppState>,
    http_req: HttpRequest,
) -> impl Responder {
    if let Err(resp) = require_write_config_scope(&http_req) {
        return resp;
    }
    let device_id = path.into_inner();
    if let Err(resp) =
        crate::api::device_pairing::require_device_owner(&http_req, &app_state, &device_id).await
    {
        return resp;
    }
    let mut config = payload.into_inner();
    config.device_id = device_id.clone();
    config.last_updated = Utc::now();
    if let Err(e) = crate::db::postgres::upsert_device_config(&app_state.pg_pool, &config).await {
        error!("Failed to update base config in DB: {:?}", e);
        return HttpResponse::InternalServerError()
            .json(json!({"error": "Failed to save configuration"}));
    }

    let audit_event = NewSystemEventRecord {
        device_id: device_id.clone(),
        level: "info".to_string(),
        category: "user_action".to_string(),
        title: "Cập nhật cấu hình cơ bản".to_string(),
        message: format!("Người dùng đã cập nhật cấu hình cơ bản cho trạm {device_id}."),
        reason: Some("config_update".to_string()),
        metadata: Some(serde_json::json!({
            "event_type": "config_change",
            "scope": "device",
            "ec_target": config.ec_target,
            "ph_target": config.ph_target,
            "control_mode": config.control_mode,
            "is_enabled": config.is_enabled,
        })),
        timestamp: config.last_updated.timestamp_millis(),
        source: "rule".to_string(),
        primary_reason_code: None,
    };
    let _ = insert_system_event(&app_state.pg_pool, &audit_event).await;

    match sync_config_to_esp32(&app_state, &device_id).await {
        Ok(()) => HttpResponse::Ok().json(json!({"status": "success"})),
        Err(e) => {
            error!("Lưu DB thành công nhưng lỗi đồng bộ config: {}", e);
            HttpResponse::Accepted().json(json!({
                "status": "partial_success",
                "message": "Đã lưu CSDL nhưng chưa đồng bộ được cấu hình tới thiết bị."
            }))
        }
    }
}

#[instrument(skip(app_state))]
pub async fn get_water_config(
    path: web::Path<String>,
    http_req: HttpRequest,
    app_state: web::Data<AppState>,
) -> impl Responder {
    if let Err(resp) = require_read_telemetry_scope(&http_req) {
        return resp;
    }
    let device_id = path.into_inner();
    let result =
        sqlx::query_as::<_, WaterConfig>("SELECT * FROM water_config WHERE device_id = $1")
            .bind(device_id)
            .fetch_optional(&app_state.pg_pool)
            .await;
    match result {
        Ok(Some(config)) => HttpResponse::Ok().json(config),
        Ok(None) => HttpResponse::NotFound().json(json!({"error": "Not found"})),
        Err(_) => HttpResponse::InternalServerError().json(json!({"error": "DB Error"})),
    }
}

#[instrument(skip(app_state, req))]
pub async fn update_water_config(
    path: web::Path<String>,
    req: web::Json<WaterConfig>,
    app_state: web::Data<AppState>,
    http_req: HttpRequest,
) -> impl Responder {
    if let Err(resp) = require_write_config_scope(&http_req) {
        return resp;
    }
    let device_id = path.into_inner();
    if let Err(resp) =
        crate::api::device_pairing::require_device_owner(&http_req, &app_state, &device_id).await
    {
        return resp;
    }
    let mut config = req.into_inner();
    config.device_id = device_id.clone();
    let now = Utc::now();
    if upsert_water_db(&app_state.pg_pool, &config, &now)
        .await
        .is_err()
    {
        return HttpResponse::InternalServerError().json(json!({"error": "DB Error"}));
    }
    match sync_config_to_esp32(&app_state, &device_id).await {
        Ok(()) => HttpResponse::Ok().json(json!({"status": "success"})),
        Err(e) => {
            error!("Lưu water config thành công nhưng lỗi đồng bộ: {}", e);
            HttpResponse::Accepted().json(json!({
                "status": "partial_success",
                "message": "Đã lưu CSDL nhưng chưa đồng bộ được cấu hình tới thiết bị."
            }))
        }
    }
}

#[instrument(skip(app_state))]
pub async fn get_safety_config(
    path: web::Path<String>,
    http_req: HttpRequest,
    app_state: web::Data<AppState>,
) -> impl Responder {
    if let Err(resp) = require_read_telemetry_scope(&http_req) {
        return resp;
    }
    let device_id = path.into_inner();
    let result =
        sqlx::query_as::<_, SafetyConfig>("SELECT * FROM safety_config WHERE device_id = $1")
            .bind(device_id)
            .fetch_optional(&app_state.pg_pool)
            .await;
    match result {
        Ok(Some(config)) => HttpResponse::Ok().json(config),
        Ok(None) => HttpResponse::NotFound().json(json!({"error": "Not found"})),
        Err(_) => HttpResponse::InternalServerError().json(json!({"error": "DB Error"})),
    }
}

#[instrument(skip(app_state, req))]
pub async fn update_safety_config(
    path: web::Path<String>,
    req: web::Json<SafetyConfig>,
    app_state: web::Data<AppState>,
    http_req: HttpRequest,
) -> impl Responder {
    if let Err(resp) = require_write_config_scope(&http_req) {
        return resp;
    }
    let device_id = path.into_inner();
    if let Err(resp) =
        crate::api::device_pairing::require_device_owner(&http_req, &app_state, &device_id).await
    {
        return resp;
    }
    let mut config = req.into_inner();
    config.device_id = device_id.clone();
    config.last_updated = Utc::now();
    if crate::db::postgres::upsert_safety_config(&app_state.pg_pool, &config)
        .await
        .is_err()
    {
        return HttpResponse::InternalServerError().json(json!({"error": "DB Error"}));
    }

    let audit_event = NewSystemEventRecord {
        device_id: device_id.clone(),
        level: "info".to_string(),
        category: "user_action".to_string(),
        title: "Cập nhật cấu hình an toàn".to_string(),
        message: format!("Người dùng đã cập nhật cấu hình an toàn cho trạm {device_id}."),
        reason: Some("config_update".to_string()),
        metadata: Some(serde_json::json!({
            "event_type": "config_change",
            "scope": "safety",
            "min_ec_limit": config.min_ec_limit,
            "max_ec_limit": config.max_ec_limit,
            "min_ph_limit": config.min_ph_limit,
            "max_ph_limit": config.max_ph_limit,
            "emergency_shutdown": config.emergency_shutdown,
        })),
        timestamp: config.last_updated.timestamp_millis(),
        source: "rule".to_string(),
        primary_reason_code: None,
    };
    let _ = insert_system_event(&app_state.pg_pool, &audit_event).await;

    match sync_config_to_esp32(&app_state, &device_id).await {
        Ok(()) => HttpResponse::Ok().json(json!({"status": "success"})),
        Err(e) => {
            error!("Lưu safety config thành công nhưng lỗi đồng bộ: {}", e);
            HttpResponse::Accepted().json(json!({
                "status": "partial_success",
                "message": "Đã lưu CSDL nhưng chưa đồng bộ được cấu hình tới thiết bị."
            }))
        }
    }
}

#[instrument(skip(app_state))]
pub async fn get_sensor_calibration(
    path: web::Path<String>,
    http_req: HttpRequest,
    app_state: web::Data<AppState>,
) -> impl Responder {
    if let Err(resp) = require_read_telemetry_scope(&http_req) {
        return resp;
    }
    let device_id = path.into_inner();
    let result = sqlx::query_as::<_, SensorCalibration>(
        "SELECT * FROM sensor_calibration WHERE device_id = $1",
    )
    .bind(device_id)
    .fetch_optional(&app_state.pg_pool)
    .await;
    match result {
        Ok(Some(config)) => HttpResponse::Ok().json(config),
        Ok(None) => HttpResponse::NotFound().json(json!({"error": "Not found"})),
        Err(_) => HttpResponse::InternalServerError().json(json!({"error": "DB Error"})),
    }
}

#[instrument(skip(app_state, req))]
pub async fn update_sensor_calibration(
    path: web::Path<String>,
    req: web::Json<SensorCalibration>,
    app_state: web::Data<AppState>,
    http_req: HttpRequest,
) -> impl Responder {
    if let Err(resp) = require_write_config_scope(&http_req) {
        return resp;
    }
    let device_id = path.into_inner();
    if let Err(resp) =
        crate::api::device_pairing::require_device_owner(&http_req, &app_state, &device_id).await
    {
        return resp;
    }
    let mut config = req.into_inner();
    config.device_id = device_id.clone();
    let now = Utc::now();
    if upsert_sensor_db(&app_state.pg_pool, &config, &now)
        .await
        .is_err()
    {
        return HttpResponse::InternalServerError().json(json!({"error": "DB Error"}));
    }
    match sync_config_to_esp32(&app_state, &device_id).await {
        Ok(()) => HttpResponse::Ok().json(json!({"status": "success"})),
        Err(e) => {
            error!("Lưu sensor config thành công nhưng lỗi đồng bộ: {}", e);
            HttpResponse::Accepted().json(json!({
                "status": "partial_success",
                "message": "Đã lưu CSDL nhưng chưa đồng bộ được cấu hình tới thiết bị."
            }))
        }
    }
}

#[instrument(skip(app_state, req))]
pub async fn finish_sensor_calibration(
    path: web::Path<String>,
    req: web::Json<FinishCalibrationRequest>,
    app_state: web::Data<AppState>,
) -> impl Responder {
    let device_id = path.into_inner();
    let payload = req.into_inner();
    let now = payload.finished_at.unwrap_or_else(Utc::now);
    let mut tx = match app_state.pg_pool.begin().await {
        Ok(tx) => tx,
        Err(_) => return HttpResponse::InternalServerError().json(json!({"error": "DB Error"})),
    };

    let existing = sqlx::query_as::<_, SensorCalibration>(
        "SELECT * FROM sensor_calibration WHERE device_id = $1",
    )
    .bind(&device_id)
    .fetch_optional(&mut *tx)
    .await
    .ok()
    .flatten()
    .unwrap_or_else(|| default_sensor_calibration(&device_id, now));

    let applied = match sqlx::query(
        r#"
        INSERT INTO sensor_calibration (
            device_id, ph_v7, ph_v4, ph_v10, ph_calibration_mode, ec_factor, ec_offset, temp_offset,
            temp_compensation_beta, publish_interval, moving_average_window,
            enable_ph_sensor, enable_ec_sensor, enable_temp_sensor, enable_water_level_sensor, last_calibrated
        ) VALUES (
            $1, $2, $3, $4, $5, $6, $7, $8,
            $9, $10, $11, $12, $13, $14, $15, $16
        )
        ON CONFLICT(device_id) DO UPDATE SET
            ph_v7 = EXCLUDED.ph_v7,
            ph_v4 = EXCLUDED.ph_v4,
            ph_v10 = EXCLUDED.ph_v10,
            ph_calibration_mode = EXCLUDED.ph_calibration_mode,
            ec_factor = EXCLUDED.ec_factor,
            ec_offset = EXCLUDED.ec_offset,
            temp_offset = EXCLUDED.temp_offset,
            temp_compensation_beta = EXCLUDED.temp_compensation_beta,
            publish_interval = EXCLUDED.publish_interval,
            moving_average_window = EXCLUDED.moving_average_window,
            enable_ph_sensor = EXCLUDED.enable_ph_sensor,
            enable_ec_sensor = EXCLUDED.enable_ec_sensor,
            enable_temp_sensor = EXCLUDED.enable_temp_sensor,
            enable_water_level_sensor = EXCLUDED.enable_water_level_sensor,
            last_calibrated = EXCLUDED.last_calibrated
        WHERE sensor_calibration.last_calibrated <= EXCLUDED.last_calibrated
        "#,
    )
    .bind(&device_id)
    .bind(payload.ph_v7)
    .bind(payload.ph_v4)
    .bind(payload.ph_v10)
    .bind(&payload.mode)
    .bind(existing.ec_factor)
    .bind(existing.ec_offset)
    .bind(existing.temp_offset)
    .bind(existing.temp_compensation_beta)
    .bind(existing.publish_interval)
    .bind(existing.moving_average_window)
    .bind(existing.enable_ph_sensor)
    .bind(existing.enable_ec_sensor)
    .bind(existing.enable_temp_sensor)
    .bind(existing.enable_water_level_sensor)
    .bind(now)
    .execute(&mut *tx)
    .await
    {
        Ok(result) => result.rows_affected() > 0,
        Err(_) => {
            return HttpResponse::InternalServerError().json(json!({"error": "DB Error"}));
        }
    };

    if applied {
        let msg = if let Some(v10) = payload.ph_v10 {
            format!(
                "Hiệu chuẩn thành công (mode: {}). pH_V7={:.4}, pH_V4={:.4}, pH_V10={:.4}, sai số={:.4}",
                payload.mode, payload.ph_v7, payload.ph_v4, v10, payload.error
            )
        } else {
            format!(
                "Hiệu chuẩn thành công (mode: {}). pH_V7={:.4}, pH_V4={:.4}, sai số={:.4}",
                payload.mode, payload.ph_v7, payload.ph_v4, payload.error
            )
        };

        let event = NewSystemEventRecord {
            device_id: device_id.clone(),
            level: "success".to_string(),
            category: "calibration".to_string(),
            title: "Hoàn tất hiệu chuẩn pH".to_string(),
            message: msg,
            reason: None,
            metadata: Some(json!({
                "event_type": "ph_calibration",
                "mode": payload.mode,
                "sample_points": payload.sample_points,
                "result": {
                    "ph_v7": payload.ph_v7,
                    "ph_v4": payload.ph_v4,
                    "ph_v10": payload.ph_v10
                },
                "error": payload.error,
                "finished_at": now
            })),
            timestamp: now.timestamp_millis(),
            source: "rule".to_string(),
            primary_reason_code: None,
        };

        if insert_system_event(&mut *tx, &event).await.is_err() {
            return HttpResponse::InternalServerError().json(json!({"error": "DB Error"}));
        }
    }

    if tx.commit().await.is_err() {
        return HttpResponse::InternalServerError().json(json!({"error": "DB Error"}));
    }

    if applied {
        match sync_config_to_esp32(&app_state, &device_id).await {
            Ok(()) => HttpResponse::Ok().json(json!({"status": "success", "applied": true})),
            Err(e) => {
                error!("Lưu calibration thành công nhưng lỗi đồng bộ: {}", e);
                HttpResponse::Accepted().json(json!({
                    "status": "partial_success",
                    "applied": true,
                    "message": "Đã lưu CSDL nhưng chưa đồng bộ được cấu hình tới thiết bị."
                }))
            }
        }
    } else {
        HttpResponse::Ok().json(json!({
            "status": "ignored_stale_request",
            "applied": false
        }))
    }
}

#[instrument(skip(app_state))]
pub async fn get_sensor_calibration_history(
    path: web::Path<String>,
    http_req: HttpRequest,
    app_state: web::Data<AppState>,
) -> impl Responder {
    if let Err(resp) = require_read_telemetry_scope(&http_req) {
        return resp;
    }
    let device_id = path.into_inner();
    let result = sqlx::query_as::<_, SystemEventRecord>(
        r#"
        SELECT id, device_id, level, category, title, message, reason, metadata, timestamp
        FROM system_events
        WHERE device_id = $1 AND category = 'calibration'
        ORDER BY timestamp DESC
        LIMIT 20
        "#,
    )
    .bind(device_id)
    .fetch_all(&app_state.pg_pool)
    .await;

    match result {
        Ok(events) => HttpResponse::Ok().json(json!({ "data": events })),
        Err(_) => HttpResponse::InternalServerError().json(json!({"error": "DB Error"})),
    }
}

#[instrument(skip(app_state))]
pub async fn get_dosing_calibration(
    path: web::Path<String>,
    http_req: HttpRequest,
    app_state: web::Data<AppState>,
) -> impl Responder {
    if let Err(resp) = require_read_telemetry_scope(&http_req) {
        return resp;
    }
    let device_id = path.into_inner();
    let result = sqlx::query_as::<_, DosingCalibration>(
        "SELECT * FROM dosing_calibration WHERE device_id = $1",
    )
    .bind(device_id)
    .fetch_optional(&app_state.pg_pool)
    .await;
    match result {
        Ok(Some(config)) => HttpResponse::Ok().json(config),
        Ok(None) => HttpResponse::NotFound().json(json!({"error": "Not found"})),
        Err(_) => HttpResponse::InternalServerError().json(json!({"error": "DB Error"})),
    }
}

#[instrument(skip(app_state, req))]
pub async fn update_dosing_calibration(
    path: web::Path<String>,
    req: web::Json<DosingCalibration>,
    app_state: web::Data<AppState>,
    http_req: HttpRequest,
) -> impl Responder {
    if let Err(resp) = require_write_config_scope(&http_req) {
        return resp;
    }
    let device_id = path.into_inner();
    if let Err(resp) =
        crate::api::device_pairing::require_device_owner(&http_req, &app_state, &device_id).await
    {
        return resp;
    }
    let mut config = req.into_inner();
    config.device_id = device_id.clone();
    if let Err(msg) = validate_dosing_constraints(&config) {
        return HttpResponse::BadRequest().json(json!({"error": msg}));
    }
    let now = Utc::now();
    if upsert_dosing_db(&app_state.pg_pool, &config, &now)
        .await
        .is_err()
    {
        return HttpResponse::InternalServerError().json(json!({"error": "DB Error"}));
    }
    match sync_config_to_esp32(&app_state, &device_id).await {
        Ok(()) => HttpResponse::Ok().json(json!({"status": "success"})),
        Err(e) => {
            error!("Lưu dosing config thành công nhưng lỗi đồng bộ: {}", e);
            HttpResponse::Accepted().json(json!({
                "status": "partial_success",
                "message": "Đã lưu CSDL nhưng chưa đồng bộ được cấu hình tới thiết bị."
            }))
        }
    }
}

pub fn init_routes(cfg: &mut web::ServiceConfig) {
    cfg.route("/config/unified", web::put().to(update_unified_config))
        .route("/config/unified", web::get().to(get_unified_device_config))
        .route("/config", web::get().to(get_config))
        .route("/config", web::put().to(update_config))
        // Canonical config resource paths. Keep the legacy `/safety` GET and
        // POST update routes below as compatibility aliases during migration.
        .route("/safety", web::get().to(get_safety_config))
        .route("/config/safety", web::get().to(get_safety_config))
        .route("/config/safety", web::post().to(update_safety_config))
        .route("/config/safety", web::put().to(update_safety_config))
        .route("/config/water", web::get().to(get_water_config))
        .route("/config/water", web::post().to(update_water_config))
        .route("/config/water", web::put().to(update_water_config))
        .route("/calibration/sensor", web::get().to(get_sensor_calibration))
        .route(
            "/calibration/sensor",
            web::post().to(update_sensor_calibration),
        )
        .route(
            "/calibration/sensor/finish",
            web::post().to(finish_sensor_calibration),
        )
        .route(
            "/calibration/sensor/history",
            web::get().to(get_sensor_calibration_history),
        )
        .route("/calibration/dosing", web::get().to(get_dosing_calibration))
        .route(
            "/calibration/dosing",
            web::post().to(update_dosing_calibration),
        );
}

#[cfg(test)]
mod tests {
    use super::*;
    //
    // #[test]
    // fn validate_dosing_constraints_accepts_pwm_boundaries() {
    //     let mut dose = DosingCalibration::default();
    //     dose.dosing_pwm_percent = 1;
    //     assert!(validate_dosing_constraints(&dose).is_ok());
    //
    //     dose.dosing_pwm_percent = 100;
    //     assert!(validate_dosing_constraints(&dose).is_ok());
    // }

    #[test]
    #[allow(clippy::unwrap_used)]
    fn validate_dosing_constraints_rejects_zero_capacity() {
        let dose = DosingCalibration {
            pump_a_capacity_ml_per_sec: 0.0,
            ..Default::default()
        };
        let result = validate_dosing_constraints(&dose);
        assert!(result.is_err());
        assert!(
            result
                .expect_err("expected err")
                .contains("pump_a_capacity_ml_per_sec")
        );
    }

    #[test]
    #[allow(clippy::unwrap_used)]
    fn validate_dosing_constraints_rejects_pwm_zero() {
        let dose = DosingCalibration {
            dosing_pwm_percent: 0,
            ..Default::default()
        };
        let result = validate_dosing_constraints(&dose);
        assert!(result.is_err());
        assert!(
            result
                .expect_err("expected err")
                .contains("dosing_pwm_percent")
        );
    }

    #[test]
    #[allow(clippy::unwrap_used)]
    fn validate_dosing_constraints_rejects_min_pwm_exceeding_dosing_pwm() {
        let dose = DosingCalibration {
            dosing_pwm_percent: 30,
            dosing_min_pwm_percent: 50,
            ..Default::default()
        };

        let result = validate_dosing_constraints(&dose);

        assert_eq!(
            result,
            Err("dosing_min_pwm_percent must be <= dosing_pwm_percent".to_string())
        );
    }

    #[test]
    #[allow(clippy::unwrap_used)]
    fn audit_config_records_have_expected_metadata() {
        let device_id = "test_dev_01".to_string();
        let audit_event = NewSystemEventRecord {
            device_id: device_id.clone(),
            level: "info".to_string(),
            category: "user_action".to_string(),
            title: "Cập nhật cấu hình trạm".to_string(),
            message: format!("Người dùng đã cập nhật cấu hình cho trạm {device_id}."),
            reason: Some("config_update".to_string()),
            metadata: Some(serde_json::json!({
                "event_type": "config_change",
                "scope": "unified",
                "ec_target": 1.8,
                "ph_target": 6.2,
                "control_mode": "auto",
                "is_enabled": true,
            })),
            timestamp: 1700000000000,
            source: "rule".to_string(),
            primary_reason_code: None,
        };

        assert_eq!(audit_event.category, "user_action");
        assert_eq!(audit_event.level, "info");
        assert_eq!(audit_event.reason.as_deref(), Some("config_update"));
        let meta = audit_event.metadata.unwrap();
        assert_eq!(meta["event_type"], "config_change");
        assert_eq!(meta["scope"], "unified");
        assert_eq!(meta["ec_target"], 1.8);
        assert_eq!(meta["ph_target"], 6.2);
    }

    #[actix_web::test]
    async fn canonical_safety_and_water_routes_are_registered() {
        use actix_web::{App, test};

        let app = test::init_service(
            App::new()
                .app_data(web::Data::new(crate::api::test_support::test_app_state()))
                .service(web::scope("/devices/{device_id}").configure(init_routes)),
        )
        .await;

        for (method, uri, expected) in [
            (
                actix_web::http::Method::GET,
                "/devices/device-1/config/safety",
                actix_web::http::StatusCode::FORBIDDEN,
            ),
            (
                actix_web::http::Method::PUT,
                "/devices/device-1/config/safety",
                actix_web::http::StatusCode::BAD_REQUEST,
            ),
            (
                actix_web::http::Method::GET,
                "/devices/device-1/config/water",
                actix_web::http::StatusCode::FORBIDDEN,
            ),
            (
                actix_web::http::Method::PUT,
                "/devices/device-1/config/water",
                actix_web::http::StatusCode::BAD_REQUEST,
            ),
        ] {
            let req = test::TestRequest::default()
                .method(method)
                .uri(uri)
                .to_request();
            let response = test::call_service(&app, req).await;
            // GET reaches the auth guard; PUT reaches the JSON extractor first.
            // Both statuses prove the canonical route matched before business logic.
            assert_eq!(response.status(), expected);
        }
    }

    #[actix_web::test]
    async fn legacy_safety_post_remains_registered_for_compatibility() {
        use actix_web::{App, test};

        let app = test::init_service(
            App::new()
                .app_data(web::Data::new(crate::api::test_support::test_app_state()))
                .service(web::scope("/devices/{device_id}").configure(init_routes)),
        )
        .await;
        let req = test::TestRequest::post()
            .uri("/devices/device-1/config/safety")
            .to_request();
        let response = test::call_service(&app, req).await;

        assert_eq!(response.status(), actix_web::http::StatusCode::BAD_REQUEST);
    }
}
