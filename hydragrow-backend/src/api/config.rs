use actix_web::{HttpResponse, Responder, web};
use chrono::{DateTime, Utc};
use hydragrow_shared::ControllerConfig;
use rumqttc::QoS;
use serde_json::json;
use tracing::{error, info, instrument};

use crate::AppState;
use crate::db::postgres::{NewSystemEventRecord, SystemEventRecord, insert_system_event};
use crate::models::config::{
    DeviceConfig, DosingCalibration, SafetyConfig, SensorCalibration, WaterConfig, from_db_rows,
};

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

    let water = water_res.ok().flatten().unwrap_or_else(|| WaterConfig {
        device_id: device_id.to_string(),
        ..Default::default()
    });

    let safe = safe_res.ok().flatten().unwrap_or_else(|| SafetyConfig {
        device_id: device_id.to_string(),
        ..Default::default()
    });

    let dose = dose_res
        .ok()
        .flatten()
        .unwrap_or_else(|| DosingCalibration {
            device_id: device_id.to_string(),
            ..Default::default()
        });

    let sens = sens_res
        .ok()
        .flatten()
        .unwrap_or_else(|| SensorCalibration {
            device_id: device_id.to_string(),
            ph_v7: 2.5,
            ph_v4: 1.428,
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
    // 1. GỬI CẤU HÌNH TỔNG HỢP CHO CONTROLLER NODE
    let payload = fetch_unified_config_concurrently(&app_state.pg_pool, device_id).await?;
    let mqtt_topic_controller = format!("AGITECH/{}/controller/config", device_id);
    let mqtt_bytes_controller =
        serde_json::to_vec(&payload).map_err(|e| format!("Lỗi serialize payload: {:?}", e))?;

    app_state
        .mqtt_client
        .publish(
            &mqtt_topic_controller,
            QoS::AtLeastOnce,
            true,
            mqtt_bytes_controller,
        )
        .await
        .map_err(|e| format!("Lỗi gửi MQTT Controller: {:?}", e))?;

    // 2. GỬI CẤU HÌNH CẢM BIẾN RIÊNG CHO SENSOR NODE
    let sens = sqlx::query_as::<_, SensorCalibration>(
        "SELECT * FROM sensor_calibration WHERE device_id = $1",
    )
    .bind(device_id)
    .fetch_optional(&app_state.pg_pool)
    .await
    .ok()
    .flatten();

    if let Some(sensor_config) = sens {
        let mqtt_topic_sensor = format!("AGITECH/{}/sensors/config", device_id);

        let sensor_payload = json!({
            "ph_v7": sensor_config.ph_v7,
            "ph_v4": sensor_config.ph_v4,
            "ph_v10": sensor_config.ph_v10,
            "ph_calibration_mode": sensor_config.ph_calibration_mode,
            "ec_factor": sensor_config.ec_factor,
            "ec_offset": sensor_config.ec_offset,
            "temp_offset": sensor_config.temp_offset,
            "temp_compensation_beta": sensor_config.temp_compensation_beta,
            "moving_average_window": sensor_config.moving_average_window,
            "publish_interval": sensor_config.publish_interval,
            "enable_ph_sensor": sensor_config.enable_ph_sensor,
            "enable_ec_sensor": sensor_config.enable_ec_sensor,
            "enable_temp_sensor": sensor_config.enable_temp_sensor,
            "enable_water_level_sensor": sensor_config.enable_water_level_sensor,
            "tank_height": payload.tank_height
        });

        if let Ok(mqtt_bytes_sensor) = serde_json::to_vec(&sensor_payload) {
            app_state
                .mqtt_client
                .publish(
                    &mqtt_topic_sensor,
                    QoS::AtLeastOnce,
                    true,
                    mqtt_bytes_sensor,
                )
                .await
                .map_err(|e| format!("Lỗi gửi MQTT Sensor: {:?}", e))?;
        }
    }

    info!(
        "✅ Đã đồng bộ cấu hình FULL xuống Controller Node & Sensor Node ({})",
        device_id
    );
    Ok(())
}

async fn upsert_water_db(
    pool: &sqlx::PgPool,
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
            misting_on_duration_ms, misting_off_duration_ms, last_updated
        ) VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9, $10, $11, $12, $13, $14, $15, $16, $17)
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
    .bind(now)
    .execute(pool)
    .await?;
    Ok(())
}

async fn upsert_sensor_db(
    pool: &sqlx::PgPool,
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
    .execute(pool).await?;
    Ok(())
}

fn default_sensor_calibration(device_id: &str, now: DateTime<Utc>) -> SensorCalibration {
    SensorCalibration {
        device_id: device_id.to_string(),
        ph_v7: 2.5,
        ph_v4: 1.428,
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

const MAX_SCHEDULED_DOSING_DURATION_SEC: f32 = 120.0;

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

    if dose.scheduled_dosing_enabled {
        let pwm_ratio = dose.dosing_pwm_percent as f32 / 100.0;
        
        let effective_capacity_a = dose.pump_a_capacity_ml_per_sec * pwm_ratio;
        let expected_duration_a_sec = dose.scheduled_dose_a_ml / effective_capacity_a;
        if expected_duration_a_sec > MAX_SCHEDULED_DOSING_DURATION_SEC {
            return Err(format!(
                "scheduled_dose_a_ml causes expected_duration_sec={:.2}s > {:.0}s safety limit",
                expected_duration_a_sec, MAX_SCHEDULED_DOSING_DURATION_SEC
            ));
        }

        let effective_capacity_b = dose.pump_b_capacity_ml_per_sec * pwm_ratio;
        let expected_duration_b_sec = dose.scheduled_dose_b_ml / effective_capacity_b;
        if expected_duration_b_sec > MAX_SCHEDULED_DOSING_DURATION_SEC {
            return Err(format!(
                "scheduled_dose_b_ml causes expected_duration_sec={:.2}s > {:.0}s safety limit",
                expected_duration_b_sec, MAX_SCHEDULED_DOSING_DURATION_SEC
            ));
        }
    }

    Ok(())
}

async fn upsert_dosing_db(
    pool: &sqlx::PgPool,
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
            scheduled_dosing_enabled, scheduled_dosing_cron, scheduled_dose_a_ml, scheduled_dose_b_ml,
            dosing_min_pwm_percent, pump_a_min_pwm_percent, pump_b_min_pwm_percent,
            pump_ph_up_min_pwm_percent, pump_ph_down_min_pwm_percent, dosing_pulse_on_ms,
            dosing_pulse_off_ms, dosing_min_dose_ml, dosing_max_pulse_count_per_cycle
        ) VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9, $10, $11, $12, $13, $14, $15, $16, $17, $18, $19, $20, $21, $22, $23, $24, $25, $26, $27, $28, $29, $30, $31, $32)
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
            scheduled_dosing_enabled = EXCLUDED.scheduled_dosing_enabled,
            scheduled_dosing_cron = EXCLUDED.scheduled_dosing_cron,
            scheduled_dose_a_ml = EXCLUDED.scheduled_dose_a_ml,
            scheduled_dose_b_ml = EXCLUDED.scheduled_dose_b_ml,
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
    .bind(cal.scheduled_dosing_enabled) 
    .bind(&cal.scheduled_dosing_cron) 
    .bind(cal.scheduled_dose_a_ml) 
    .bind(cal.scheduled_dose_b_ml) 
    .bind(cal.dosing_min_pwm_percent) 
    .bind(cal.pump_a_min_pwm_percent) 
    .bind(cal.pump_b_min_pwm_percent) 
    .bind(cal.pump_ph_up_min_pwm_percent) 
    .bind(cal.pump_ph_down_min_pwm_percent) 
    .bind(cal.dosing_pulse_on_ms) 
    .bind(cal.dosing_pulse_off_ms) 
    .bind(cal.dosing_min_dose_ml) 
    .bind(cal.dosing_max_pulse_count_per_cycle) 
    .execute(pool).await?;

    Ok(())
}

#[instrument(skip(app_state, req))]
pub async fn update_unified_config(
    path: web::Path<String>,
    req: web::Json<UnifiedConfigRequest>,
    app_state: web::Data<AppState>,
) -> impl Responder {
    let device_id = path.into_inner();
    let mut payload = req.into_inner();
    let now = Utc::now();

    payload.device_config.device_id = device_id.clone();
    payload.device_config.last_updated = now;
    if let Err(e) = crate::db::postgres::upsert_device_config(&app_state.pg_pool, &payload.device_config).await {
        error!("Failed to update device config: {:?}", e);
        return HttpResponse::InternalServerError().json(json!({"error": "DB Error: Device"}));
    }

    payload.safety_config.device_id = device_id.clone();
    payload.safety_config.last_updated = now.clone();
    if let Err(e) = crate::db::postgres::upsert_safety_config(&app_state.pg_pool, &payload.safety_config).await {
        error!("Failed to update safety config: {:?}", e);
        return HttpResponse::InternalServerError().json(json!({"error": "DB Error: Safety"}));
    }

    payload.water_config.device_id = device_id.clone();
    if let Err(e) = upsert_water_db(&app_state.pg_pool, &payload.water_config, &now).await {
        error!("Failed to update water config: {:?}", e);
        return HttpResponse::InternalServerError().json(json!({"error": "DB Error: Water"}));
    }

    payload.sensor_calibration.device_id = device_id.clone();
    if let Err(e) = upsert_sensor_db(&app_state.pg_pool, &payload.sensor_calibration, &now).await {
        error!("Failed to update sensor config: {:?}", e);
        return HttpResponse::InternalServerError().json(json!({"error": "DB Error: Sensor"}));
    }

    payload.dosing_calibration.device_id = device_id.clone();
    if let Err(msg) = validate_dosing_constraints(&payload.dosing_calibration) {
        return HttpResponse::BadRequest().json(json!({"error": msg}));
    }
    if let Err(e) = upsert_dosing_db(&app_state.pg_pool, &payload.dosing_calibration, &now).await {
        error!("Failed to update dosing config: {:?}", e);
        return HttpResponse::InternalServerError().json(json!({"error": "DB Error: Dosing"}));
    }

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
) -> impl Responder {
    let device_id = path.into_inner();
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

    let response_payload = UnifiedConfigRequest {
        device_config: dev_res.ok().flatten().unwrap_or_else(|| DeviceConfig {
            device_id: device_id.clone(),
            ec_target: 1.5,
            ec_tolerance: 0.1,
            ph_target: 6.0,
            ph_tolerance: 0.5,
            temp_target: 25.0,
            temp_tolerance: 2.0,
            control_mode: "auto".to_string(),
            is_enabled: false,
            delay_between_a_and_b_sec: 10,
            last_updated: Utc::now(),
        }),
        water_config: water_res.ok().flatten().unwrap_or_else(|| WaterConfig {
            device_id: device_id.clone(),
            ..Default::default()
        }),
        safety_config: safe_res.ok().flatten().unwrap_or_else(|| SafetyConfig {
            device_id: device_id.clone(),
            ..Default::default()
        }),
        dosing_calibration: dose_res
            .ok()
            .flatten()
            .unwrap_or_else(|| DosingCalibration {
                device_id: device_id.clone(),
                ..Default::default()
            }),
        sensor_calibration: sens_res
            .ok()
            .flatten()
            .unwrap_or_else(|| SensorCalibration {
                device_id: device_id.clone(),
                ph_v7: 2.5,
                ph_v4: 1.428,
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
            }),
    };

    HttpResponse::Ok().json(response_payload)
}

#[instrument(skip(app_state))]
pub async fn get_config(path: web::Path<String>, app_state: web::Data<AppState>) -> impl Responder {
    let device_id = path.into_inner();
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
) -> impl Responder {
    let device_id = path.into_inner();
    let mut config = payload.into_inner();
    config.device_id = device_id.clone();
    config.last_updated = Utc::now();
    if let Err(e) = crate::db::postgres::upsert_device_config(&app_state.pg_pool, &config).await {
        error!("Failed to update base config in DB: {:?}", e);
        return HttpResponse::InternalServerError()
            .json(json!({"error": "Failed to save configuration"}));
    }
    let _ = sync_config_to_esp32(&app_state, &device_id).await;
    HttpResponse::Ok().json(json!({"status": "success"}))
}

#[instrument(skip(app_state))]
pub async fn get_water_config(
    path: web::Path<String>,
    app_state: web::Data<AppState>,
) -> impl Responder {
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
) -> impl Responder {
    let device_id = path.into_inner();
    let config = req.into_inner();
    let now = Utc::now();
    if let Err(_) = upsert_water_db(&app_state.pg_pool, &config, &now).await {
        return HttpResponse::InternalServerError().json(json!({"error": "DB Error"}));
    }
    let _ = sync_config_to_esp32(&app_state, &device_id).await;
    HttpResponse::Ok().json(json!({"status": "success"}))
}

#[instrument(skip(app_state))]
pub async fn get_safety_config(
    path: web::Path<String>,
    app_state: web::Data<AppState>,
) -> impl Responder {
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
) -> impl Responder {
    let device_id = path.into_inner();
    let mut config = req.into_inner();
    config.device_id = device_id.clone();
    config.last_updated = Utc::now();
    if let Err(_) = crate::db::postgres::upsert_safety_config(&app_state.pg_pool, &config).await {
        return HttpResponse::InternalServerError().json(json!({"error": "DB Error"}));
    }
    let _ = sync_config_to_esp32(&app_state, &device_id).await;
    HttpResponse::Ok().json(json!({"status": "success"}))
}

#[instrument(skip(app_state))]
pub async fn get_sensor_calibration(
    path: web::Path<String>,
    app_state: web::Data<AppState>,
) -> impl Responder {
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
) -> impl Responder {
    let device_id = path.into_inner();
    let config = req.into_inner();
    let now = Utc::now();
    if let Err(_) = upsert_sensor_db(&app_state.pg_pool, &config, &now).await {
        return HttpResponse::InternalServerError().json(json!({"error": "DB Error"}));
    }
    let _ = sync_config_to_esp32(&app_state, &device_id).await;
    HttpResponse::Ok().json(json!({"status": "success"}))
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
        };

        if insert_system_event(&mut *tx, &event).await.is_err() {
            return HttpResponse::InternalServerError().json(json!({"error": "DB Error"}));
        }
    }

    if tx.commit().await.is_err() {
        return HttpResponse::InternalServerError().json(json!({"error": "DB Error"}));
    }

    if applied {
        let _ = sync_config_to_esp32(&app_state, &device_id).await;
        HttpResponse::Ok().json(json!({"status": "success", "applied": true}))
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
    app_state: web::Data<AppState>,
) -> impl Responder {
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
    app_state: web::Data<AppState>,
) -> impl Responder {
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
) -> impl Responder {
    let device_id = path.into_inner();
    let mut config = req.into_inner();
    config.device_id = device_id.clone();
    if let Err(msg) = validate_dosing_constraints(&config) {
        return HttpResponse::BadRequest().json(json!({"error": msg}));
    }
    let now = Utc::now();
    if let Err(_) = upsert_dosing_db(&app_state.pg_pool, &config, &now).await {
        return HttpResponse::InternalServerError().json(json!({"error": "DB Error"}));
    }
    let _ = sync_config_to_esp32(&app_state, &device_id).await;
    HttpResponse::Ok().json(json!({"status": "success"}))
}

pub fn init_routes(cfg: &mut web::ServiceConfig) {
    cfg.route("/config/unified", web::put().to(update_unified_config))
        .route("/config/unified", web::get().to(get_unified_device_config))
        .route("/config", web::get().to(get_config))
        .route("/config", web::put().to(update_config))
        .route("/safety", web::get().to(get_safety_config))
        .route("/config/safety", web::post().to(update_safety_config))
        .route("/config/water", web::get().to(get_water_config))
        .route("/config/water", web::post().to(update_water_config))
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

    #[test]
    fn validate_dosing_constraints_accepts_pwm_boundaries() {
        let mut dose = DosingCalibration::default();
        dose.dosing_pwm_percent = 1;
        assert!(validate_dosing_constraints(&dose).is_ok());

        dose.dosing_pwm_percent = 100;
        assert!(validate_dosing_constraints(&dose).is_ok());
    }

    #[test]
    fn validate_dosing_constraints_rejects_zero_capacity() {
        let mut dose = DosingCalibration::default();
        dose.pump_a_capacity_ml_per_sec = 0.0;
        let result = validate_dosing_constraints(&dose);
        assert!(result.is_err());
        assert!(result.unwrap_err().contains("pump_a_capacity_ml_per_sec"));
    }

    #[test]
    fn validate_dosing_constraints_rejects_pwm_zero() {
        let mut dose = DosingCalibration::default();
        dose.dosing_pwm_percent = 0;
        let result = validate_dosing_constraints(&dose);
        assert!(result.is_err());
        assert!(result.unwrap_err().contains("dosing_pwm_percent"));
    }

    #[test]
    fn validate_dosing_constraints_rejects_too_long_scheduled_duration() {
        let mut dose = DosingCalibration::default();
        dose.scheduled_dosing_enabled = true;
        dose.dosing_pwm_percent = 50;
        dose.pump_a_capacity_ml_per_sec = 1.0;
        dose.scheduled_dose_a_ml = 121.0;
        let result = validate_dosing_constraints(&dose);
        assert!(result.is_err());
        assert!(result.unwrap_err().contains("scheduled_dose_a_ml"));
    }
}
