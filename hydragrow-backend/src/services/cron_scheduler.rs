use chrono::{DateTime, Utc};
use chrono_tz::Tz;
use serde_json::{Value, json};
use std::collections::HashMap;
use std::str::FromStr;
use std::time::Duration;
use tracing::{error, info, warn};
use uuid::Uuid;

use crate::metrics::SAFETY_DECISIONS_TOTAL;

const CRON_SENSOR_MAX_AGE: Duration = Duration::from_secs(300);

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum CronInputQuality {
    KnownValid,
    Unknown,
    Stale,
    Invalid,
}

fn scheduler_operational_gate(
    state: &hydragrow_shared::telemetry::OperationalState,
) -> Result<(), &'static str> {
    if state.contact != hydragrow_shared::telemetry::ContactState::Contacted {
        return Err("CONTROLLER_CONTACT_UNKNOWN");
    }
    if state.freshness != hydragrow_shared::telemetry::FreshnessState::Fresh {
        return Err("CONTROLLER_STATE_STALE");
    }
    Ok(())
}

async fn current_operational_gate(
    app_state: &crate::AppState,
    device_id: &str,
) -> Result<(), &'static str> {
    let raw = app_state
        .device_states
        .read()
        .await
        .get(device_id)
        .cloned()
        .ok_or("CONTROLLER_CONTACT_UNKNOWN")?;
    let state = serde_json::from_str::<serde_json::Value>(&raw)
        .ok()
        .and_then(|value| value.get("telemetry").cloned())
        .and_then(|value| {
            serde_json::from_value::<hydragrow_shared::telemetry::AuthoritativeTelemetrySnapshot>(
                value,
            )
            .ok()
        })
        .map(|mut snapshot| {
            snapshot.refresh_operational_state(Utc::now());
            snapshot.operational_state
        })
        .ok_or("CONTROLLER_CONTACT_UNKNOWN")?;
    scheduler_operational_gate(&state)
}

fn cron_sensor_quality(
    latest: &crate::models::sensor::SensorData,
    now: DateTime<Utc>,
) -> CronInputQuality {
    if latest.device_id.trim().is_empty()
        || !latest.ph.is_finite()
        || !latest.ec.is_finite()
        || !latest.temp.is_finite()
        || !latest.water_level.is_finite()
    {
        return CronInputQuality::Invalid;
    }
    if latest.err_ph == Some(true)
        || latest.err_ec == Some(true)
        || latest.err_temp == Some(true)
        || latest.err_water == Some(true)
    {
        return CronInputQuality::Invalid;
    }
    let Ok(observed_at) = DateTime::parse_from_rfc3339(&latest.time) else {
        return CronInputQuality::Unknown;
    };
    let age = now.signed_duration_since(observed_at.with_timezone(&Utc));
    if age < chrono::Duration::zero() || age.to_std().map_or(true, |age| age > CRON_SENSOR_MAX_AGE)
    {
        return CronInputQuality::Stale;
    }
    CronInputQuality::KnownValid
}

/// Payload cho eval_with_dynamic_map — PHẢI khớp đúng bộ key mà
/// ScriptEngine::eval_alert_with_context dùng cho đường sensor-tick thật
/// (ph/ec/temp/water_level/device_id/timestamp_ms), vì mọi Rhai script được
/// compile ra cùng 1 chữ ký `fn main(payload)` bất kể trigger là gì.
fn build_cron_payload(
    latest: &crate::models::sensor::SensorData,
    device_id: &str,
    observed_at: DateTime<Utc>,
) -> serde_json::Map<String, serde_json::Value> {
    let mut payload = serde_json::Map::new();
    payload.insert("ph".to_string(), json!(latest.ph));
    payload.insert("ec".to_string(), json!(latest.ec));
    payload.insert("temp".to_string(), json!(latest.temp));
    payload.insert("water_level".to_string(), json!(latest.water_level));
    payload.insert("device_id".to_string(), json!(device_id));
    payload.insert(
        "timestamp_ms".to_string(),
        json!(observed_at.timestamp_millis()),
    );
    payload
}

pub fn compute_next_run(expression: &str, from: DateTime<Tz>) -> Result<DateTime<Tz>, String> {
    let schedule = cron::Schedule::from_str(expression)
        .map_err(|e| format!("Cron expression không hợp lệ: {}", e))?;
    schedule
        .after(&from)
        .next()
        .ok_or_else(|| "Không tính được lần chạy tiếp theo".to_string())
}

/// Trigger type mà Flow khai báo trong `ir_json.trigger.type`. `None` khi
/// script viết tay hoặc IR không có trigger (back-compat: coi như sensor).
fn trigger_type_of_ir(ir: &serde_json::Value) -> Option<&str> {
    ir.pointer("/trigger/type").and_then(|v| v.as_str())
}

/// Lịch cron của một Flow, rút trích từ `ir_json.trigger`.
#[derive(Debug, Clone, PartialEq)]
pub enum CronSchedule {
    /// Flow là cron trigger — kèm expression + timezone đã parse.
    Cron { expression: String, tz: Tz },
    /// Flow KHÔNG phải cron trigger (sensor/fsm/webhook/hand-written) —
    /// `cron_next_run_at` phải về NULL.
    NotCron,
}

/// Phân tích lịch cron từ IR. `Err` = Flow *có* trigger cron nhưng expression
/// hoặc timezone không hợp lệ (không thể schedule được).
pub fn cron_schedule_from_ir(ir: &serde_json::Value) -> Result<CronSchedule, String> {
    match trigger_type_of_ir(ir) {
        Some("cron") => {}
        _ => return Ok(CronSchedule::NotCron),
    }
    let expression = ir
        .pointer("/trigger/cronExpression")
        .and_then(|v| v.as_str())
        .filter(|s| !s.trim().is_empty())
        .ok_or_else(|| "Thiếu trigger.cronExpression".to_string())?
        .to_string();
    let tz_name = ir
        .pointer("/trigger/timezone")
        .and_then(|v| v.as_str())
        .unwrap_or("Asia/Ho_Chi_Minh");
    let tz = tz_name
        .parse::<Tz>()
        .map_err(|e| format!("Timezone không hợp lệ: {}", e))?;
    // Validate expression ngay tại đây — cần nó hợp lệ để bootstrap lần chạy
    // đầu (F2) và để write-time validation (F2) trả 400 sớm.
    cron::Schedule::from_str(&expression)
        .map_err(|e| format!("Cron expression không hợp lệ: {}", e))?;
    Ok(CronSchedule::Cron { expression, tz })
}

/// Validation khi GHI script (create/update/template): Flow không phải cron
/// luôn hợp lệ; Flow cron bắt buộc có expression + timezone parse được.
pub fn validate_cron_trigger(ir_json: Option<&Value>) -> Result<(), String> {
    match ir_json {
        None => Ok(()),
        Some(ir) => match cron_schedule_from_ir(ir) {
            Ok(_) => Ok(()),
            Err(e) => Err(e),
        },
    }
}

async fn set_cron_next_run(
    pool: &sqlx::PgPool,
    script_id: Uuid,
    next: Option<DateTime<Utc>>,
    disable: Option<bool>,
) {
    let _ = sqlx::query(
        "UPDATE user_scripts SET cron_next_run_at = $1, enabled = COALESCE($2, enabled) WHERE id = $3",
    )
    .bind(next)
    .bind(disable)
    .bind(script_id)
    .execute(pool)
    .await;
}

/// BOOTSTRAP `cron_next_run_at` — điền định kỳ khi tạo/update/template-clone
/// script để lấp lỗ hổng "cron proxy NULL → không bao giờ đáo hạn lần đầu".
///
/// Quy tắc (giữ nguyên bước nhảy thời gian, không re-fire vô hạn):
///  - không phải cron → SET NULL, giữ nguyên `enabled`.
///  - cron đang có next nằm trong TƯƠNG LAI → giữ nguyên (chưa tới hạn).
///  - cron NULL / đã quá hạn → next = lần chạy kế tiếp kể từ max(now, prev).
///  - schedule lỗi (write-time đã chặn, đây chỉ phòng hờ) → NULL + disable.
pub async fn sync_script_cron_next_run(
    pool: &sqlx::PgPool,
    script_id: Uuid,
    ir_json: Option<&Value>,
    prev_next: Option<DateTime<Utc>>,
) {
    let Some(ir) = ir_json else {
        set_cron_next_run(pool, script_id, None, None).await;
        return;
    };
    match cron_schedule_from_ir(ir) {
        Ok(CronSchedule::NotCron) => set_cron_next_run(pool, script_id, None, None).await,
        Ok(CronSchedule::Cron { expression, tz }) => {
            let now = Utc::now().with_timezone(&tz);
            let base = prev_next
                .map(|p| p.with_timezone(&tz))
                .filter(|p| *p > now)
                .unwrap_or(now);
            match compute_next_run(&expression, base) {
                Ok(next) => {
                    set_cron_next_run(pool, script_id, Some(next.with_timezone(&Utc)), None).await
                }
                Err(e) => {
                    error!(
                        script_id = %script_id,
                        error = %e,
                        "không bootstrap được cron_next_run_at — tắt cron cho Flow"
                    );
                    set_cron_next_run(pool, script_id, None, Some(false)).await;
                }
            }
        }
        Err(e) => {
            error!(
                script_id = %script_id,
                error = %e,
                "trigger cron lỗi lúc bootstrap — tắt cron cho Flow"
            );
            set_cron_next_run(pool, script_id, None, Some(false)).await;
        }
    }
}

/// Config·Overwrite reconcile cho ĐƯỜNG CRON — mirror của khối trong
/// `eval_flow_chain` (script_eval.rs). Sau khi tách trigger theo nguồn (F4),
/// Flow cron không còn xuất hiện trong sensor-tick nên phải áp/khôi phục
/// override ở chính nơi nó chạy, không thì override đóng băng vĩnh viễn.
async fn reconcile_cron_config_overwrite(
    pool: &sqlx::PgPool,
    script: &crate::models::script::UserScript,
    payload: &serde_json::Map<String, Value>,
) {
    let Some(ir_json) = &script.ir_json else {
        return;
    };
    let Some(directive) = crate::services::config_context::parse_config_overwrite(ir_json) else {
        return;
    };

    let reads = crate::services::config_context::parse_context_reads(ir_json);
    let context =
        crate::services::config_context::resolve_context_reads(pool, &script.device_id, &reads)
            .await
            .unwrap_or_default();

    let mut sample: HashMap<String, crate::models::script::SampleValue> = HashMap::new();
    for key in ["ph", "ec", "temp", "water_level"] {
        if let Some(value) = payload.get(key).and_then(|v| v.as_f64()) {
            sample.insert(
                key.to_string(),
                crate::models::script::SampleValue::Value(value),
            );
        }
    }
    for (k, v) in &context {
        sample.insert(k.clone(), crate::models::script::SampleValue::Value(*v));
    }

    let conditions = ir_json
        .get("conditions")
        .and_then(|c| c.as_array())
        .cloned()
        .unwrap_or_default();
    let mut trace = Vec::new();
    let condition_state = conditions
        .iter()
        .all(|c| crate::api::script::eval_condition_tree(c, &sample, &mut trace));

    let config_key = directive.config_key.clone();
    let contenders = vec![crate::services::config_override::OverwriteContender {
        script_id: script.id,
        directive,
        condition_state,
        context,
    }];
    if let Err(e) = crate::services::config_override::reconcile_config_overwrite_group(
        pool,
        &script.device_id,
        &config_key,
        &contenders,
    )
    .await
    {
        warn!(
            script_id = %script.id,
            device_id = %script.device_id,
            config_key,
            error = %e,
            "cron: config overwrite reconcile thất bại"
        );
    }
}

/// Chạy nền, quét mỗi 30s các Flow có cron_next_run_at đã tới hạn.
pub async fn run_cron_loop(app_state: std::sync::Arc<crate::AppState>) {
    let mut ticker = tokio::time::interval(Duration::from_secs(30));
    loop {
        ticker.tick().await;
        if let Err(e) = tick_once(&app_state).await {
            error!(error = %e, "cron_scheduler tick thất bại");
        }
    }
}

async fn tick_once(app_state: &crate::AppState) -> Result<(), sqlx::Error> {
    let due: Vec<crate::models::script::UserScript> = sqlx::query_as(
        "SELECT * FROM user_scripts \
         WHERE cron_next_run_at IS NOT NULL AND cron_next_run_at <= now() AND enabled = TRUE",
    )
    .fetch_all(&app_state.pg_pool)
    .await?;

    let engine = std::sync::Arc::new(crate::services::script_engine::ScriptEngine::new());

    for script in due {
        info!(script_id = %script.id, "cron trigger fired");

        // Fetch payload MỘT lần cho cả: chain eval, config-overwrite reconcile.
        let payload = match crate::db::influx::get_latest_sensor_data(
            &app_state.influx_client,
            &app_state.influx_bucket,
            &script.device_id,
        )
        .await
        {
            Ok(latest) => {
                let observed_at = DateTime::parse_from_rfc3339(&latest.time)
                    .ok()
                    .map(|dt| dt.with_timezone(&Utc));
                match (observed_at, cron_sensor_quality(&latest, Utc::now())) {
                    (Some(observed_at), CronInputQuality::KnownValid) => {
                        Some(build_cron_payload(&latest, &script.device_id, observed_at))
                    }
                    (_, quality) => {
                        let reason = match quality {
                            CronInputQuality::Unknown => "INPUT_UNKNOWN",
                            CronInputQuality::Stale => "INPUT_STALE",
                            CronInputQuality::Invalid => "INPUT_INVALID",
                            CronInputQuality::KnownValid => unreachable!(),
                        };
                        warn!(
                            script_id = %script.id,
                            device_id = %script.device_id,
                            reason_code = reason,
                            "cron: required sensor input is not known-valid — action denied"
                        );
                        SAFETY_DECISIONS_TOTAL
                            .with_label_values(&[reason, "denied"])
                            .inc();
                        let _ = crate::services::execution_log::log_error(
                            &app_state.pg_pool,
                            script.id,
                            &script.device_id,
                            reason,
                            Some("cron_trigger"),
                            None,
                        )
                        .await;
                        None
                    }
                }
            }
            Err(e) => {
                warn!(
                    script_id = %script.id,
                    device_id = %script.device_id,
                    error = %e,
                    reason_code = "INPUT_ERROR",
                    "cron: không lấy được sensor data mới nhất — action denied"
                );
                SAFETY_DECISIONS_TOTAL
                    .with_label_values(&["INPUT_ERROR", "denied"])
                    .inc();
                let _ = crate::services::execution_log::log_error(
                    &app_state.pg_pool,
                    script.id,
                    &script.device_id,
                    "INPUT_ERROR",
                    Some("cron_trigger"),
                    None,
                )
                .await;
                None
            }
        };

        // Dispatch chain execution if AST exists
        if let Some(payload) = payload.as_ref()
            && let Ok(ast) = engine.compile(&script.source)
        {
            let node = crate::mqtt::handlers::script_eval::WebhookChainNode {
                id: script.id,
                name: script.name.clone(),
                kind: match script.kind.as_str() {
                    "alert" => crate::models::script::ScriptKind::Alert,
                    "action_command" => crate::models::script::ScriptKind::ActionCommand,
                    _ => crate::models::script::ScriptKind::Alert,
                },
                next_flow_ids: script.next_flow_ids.clone(),
                ast,
            };

            let all_nodes = vec![node];
            let results = crate::mqtt::handlers::script_eval::eval_webhook_chain(
                &engine, &all_nodes, payload,
            );

            for (_id, res) in results {
                match res {
                    crate::mqtt::handlers::script_eval::ChainFireResult::Alert(alert) => {
                        crate::db::postgres::touch_script_last_run(&app_state.pg_pool, &script.id)
                            .await;
                        let _ = crate::services::execution_log::log_success(
                            &app_state.pg_pool,
                            script.id,
                            &script.device_id,
                            Some("cron_trigger"),
                            None,
                        )
                        .await;
                        crate::mqtt::handlers::script_eval::handle_fired_alert(
                            app_state,
                            &script.id,
                            &script.name,
                            alert,
                            &script.device_id,
                            chrono::Utc::now().timestamp_millis(),
                        )
                        .await;
                    }
                    crate::mqtt::handlers::script_eval::ChainFireResult::ActionCommand(cmd) => {
                        if let Err(reason) =
                            current_operational_gate(app_state, &script.device_id).await
                        {
                            warn!(
                                script_id = %script.id,
                                device_id = %script.device_id,
                                reason_code = reason,
                                "cron action blocked: operational prerequisite is not current"
                            );
                            SAFETY_DECISIONS_TOTAL
                                .with_label_values(&[reason, "denied"])
                                .inc();
                            let _ = crate::services::execution_log::log_error(
                                &app_state.pg_pool,
                                script.id,
                                &script.device_id,
                                reason,
                                Some("cron_trigger"),
                                None,
                            )
                            .await;
                            continue;
                        }
                        let dispatch_result = match crate::services::safety_data::classify_safety_config(
                            crate::db::postgres::fetch_safety_config(&app_state.pg_pool, &script.device_id).await,
                        ) {
                            Ok(cfg) => {
                                let limits = hydragrow_shared::safety::DoseSafetyLimits {
                                    max_dose_per_cycle_ml: cfg.max_dose_per_cycle,
                                    max_dose_per_hour_ml: cfg.max_dose_per_hour,
                                    cooldown_sec: cfg.cooldown_sec as u64,
                                };
                                let calibration = crate::services::safety_data::classify_calibration(crate::db::postgres::fetch_dosing_calibration(
                                    &app_state.pg_pool,
                                    &script.device_id,
                                ).await);
                                let hourly = crate::services::safety_data::classify_history(crate::db::postgres::get_dosing_history_last_hour(
                                    &app_state.pg_pool,
                                    &script.device_id,
                                ).await);
                                let last_dose = crate::services::safety_data::classify_last_dose(crate::db::postgres::get_last_dose_at(
                                    &app_state.pg_pool,
                                    &script.device_id,
                                ).await);

                                match (calibration, hourly, last_dose) {
                                    (Ok(calibration), Ok(hourly), Ok(last_dose)) => {
                                        let now_sec =
                                            (chrono::Utc::now().timestamp_millis() / 1000) as u64;
                                        crate::services::action_dispatch::dispatch_action_command(
                                            app_state,
                                            &script.device_id,
                                            cmd,
                                            &limits,
                                            &hourly,
                                            now_sec,
                                            last_dose,
                                            Some(&calibration),
                                        )
                                        .await
                                    }
                                    (Err(reason), _, _) | (_, Err(reason), _) | (_, _, Err(reason)) => Err(
                                        crate::services::action_dispatch::ActionDispatchError::SafetyData(reason.to_string())
                                    ),
                                }
                            }
                            Err(error) => Err(crate::services::action_dispatch::ActionDispatchError::SafetyData(error.reason_code().to_string())),
                        };
                        match dispatch_result {
                            Ok(()) => {
                                SAFETY_DECISIONS_TOTAL
                                    .with_label_values(&["KNOWN_VALID", "allowed"])
                                    .inc();
                                crate::db::postgres::touch_script_last_run(
                                    &app_state.pg_pool,
                                    &script.id,
                                )
                                .await;
                                let _ = crate::services::execution_log::log_success(
                                    &app_state.pg_pool,
                                    script.id,
                                    &script.device_id,
                                    Some("cron_trigger"),
                                    None,
                                )
                                .await;
                            }
                            Err(error) => {
                                let reason = match &error {
                                    crate::services::action_dispatch::ActionDispatchError::SafetyData(reason) => reason.as_str(),
                                    _ => "SAFETY_INPUT_ERROR",
                                };
                                SAFETY_DECISIONS_TOTAL
                                    .with_label_values(&[reason, "denied"])
                                    .inc();
                                warn!(script_id = %script.id, device_id = %script.device_id, error = ?error, reason_code = reason, "cron action dispatch blocked");
                                let _ = crate::services::execution_log::log_error(
                                    &app_state.pg_pool,
                                    script.id,
                                    &script.device_id,
                                    reason,
                                    Some("cron_trigger"),
                                    None,
                                )
                                .await;
                            }
                        }
                    }
                    crate::mqtt::handlers::script_eval::ChainFireResult::RecipeOverride(_) => {}
                }
            }
        } else {
            let _ = crate::services::execution_log::log_error(
                &app_state.pg_pool,
                script.id,
                &script.device_id,
                "compile thất bại khi chạy cron trigger",
                Some("cron_trigger"),
                None,
            )
            .await;
        }

        // Config·Overwrite reconcile cho chính Flow này (F4 — sau khi tách
        // trigger theo nguồn, cron không còn được sensor-tick áp/khôi phục).
        if let Some(payload) = payload.as_ref() {
            reconcile_cron_config_overwrite(&app_state.pg_pool, &script, payload).await;
        }

        // Tính next_run SAU khi đã fire. Base = max(now, due trước đó): luôn
        // tiến về trước, nên lịch * * * * * không thể re-fire vô hạn.
        if let Some(ir) = &script.ir_json {
            match cron_schedule_from_ir(ir) {
                Ok(CronSchedule::Cron { expression, tz }) => {
                    let now = Utc::now().with_timezone(&tz);
                    let base = script
                        .cron_next_run_at
                        .map(|p| p.with_timezone(&tz))
                        .filter(|p| *p > now)
                        .unwrap_or(now);
                    match compute_next_run(&expression, base) {
                        Ok(next) => {
                            let _ = sqlx::query(
                                "UPDATE user_scripts SET cron_next_run_at = $1 WHERE id = $2",
                            )
                            .bind(next.with_timezone(&Utc))
                            .bind(script.id)
                            .execute(&app_state.pg_pool)
                            .await;
                        }
                        Err(e) => {
                            warn!(
                                script_id = %script.id,
                                error = %e,
                                "không tính được next_run — tắt cron cho Flow"
                            );
                            set_cron_next_run(&app_state.pg_pool, script.id, None, Some(false))
                                .await;
                        }
                    }
                }
                Ok(CronSchedule::NotCron) => {
                    warn!(
                        script_id = %script.id,
                        "flow tới hạn nhưng trigger không phải cron — xoá cron_next_run_at"
                    );
                    set_cron_next_run(&app_state.pg_pool, script.id, None, None).await;
                }
                Err(e) => {
                    warn!(
                        script_id = %script.id,
                        error = %e,
                        "trigger cron lỗi — tắt cron cho Flow"
                    );
                    set_cron_next_run(&app_state.pg_pool, script.id, None, Some(false)).await;
                }
            }
        }
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use chrono::TimeZone;

    #[test]
    fn build_cron_payload_maps_a_sensor_data_row_onto_the_same_keys_eval_alert_with_context_uses() {
        let latest = crate::models::sensor::SensorData {
            device_id: "dev-1".to_string(),
            ph: 6.4,
            ec: 1.9,
            temp: 25.5,
            water_level: 80.0,
            pump_status: Default::default(),
            time: "2026-09-09T07:00:00Z".to_string(),
            controller_received_ms: None,
            rssi: None,
            free_heap: None,
            uptime: None,
            err_water: None,
            err_temp: None,
            err_ph: None,
            err_ec: None,
            is_continuous: None,
            ph_voltage_mv: None,
            ec_received_ms: None,
            ph_received_ms: None,
            temp_received_ms: None,
            water_received_ms: None,
        };
        let observed_at = DateTime::parse_from_rfc3339(&latest.time)
            .unwrap()
            .with_timezone(&Utc);
        let payload = build_cron_payload(&latest, "dev-1", observed_at);
        assert_eq!(
            payload.get("ph").and_then(|v| v.as_f64()),
            Some(6.4f32 as f64)
        );
        assert_eq!(
            payload.get("ec").and_then(|v| v.as_f64()),
            Some(1.9f32 as f64)
        );
        assert_eq!(
            payload.get("temp").and_then(|v| v.as_f64()),
            Some(25.5f32 as f64)
        );
        assert_eq!(
            payload.get("water_level").and_then(|v| v.as_f64()),
            Some(80.0f32 as f64)
        );
        assert_eq!(
            payload.get("device_id").and_then(|v| v.as_str()),
            Some("dev-1")
        );
        assert!(
            payload
                .get("timestamp_ms")
                .and_then(|v| v.as_i64())
                .is_some()
        );
    }

    fn sensor_data_for_quality(time: &str) -> crate::models::sensor::SensorData {
        crate::models::sensor::SensorData {
            device_id: "dev-1".to_string(),
            ph: 6.4,
            ec: 1.9,
            temp: 25.5,
            water_level: 80.0,
            pump_status: Default::default(),
            time: time.to_string(),
            controller_received_ms: None,
            rssi: None,
            free_heap: None,
            uptime: None,
            err_water: None,
            err_temp: None,
            err_ph: None,
            err_ec: None,
            is_continuous: None,
            ph_voltage_mv: None,
            ec_received_ms: None,
            ph_received_ms: None,
            temp_received_ms: None,
            water_received_ms: None,
        }
    }

    #[test]
    fn scheduler_gate_requires_contact_and_freshness() {
        let mut state = hydragrow_shared::telemetry::OperationalState::default();
        assert_eq!(
            scheduler_operational_gate(&state),
            Err("CONTROLLER_CONTACT_UNKNOWN")
        );
        state.contact = hydragrow_shared::telemetry::ContactState::Contacted;
        assert_eq!(
            scheduler_operational_gate(&state),
            Err("CONTROLLER_STATE_STALE")
        );
        state.freshness = hydragrow_shared::telemetry::FreshnessState::Fresh;
        assert_eq!(scheduler_operational_gate(&state), Ok(()));
    }

    #[test]
    fn cron_sensor_quality_accepts_fresh_valid_observation() {
        let now = chrono::Utc::now();
        let latest = sensor_data_for_quality(&(now - chrono::Duration::seconds(60)).to_rfc3339());
        assert_eq!(
            cron_sensor_quality(&latest, now),
            CronInputQuality::KnownValid
        );
    }

    #[test]
    fn cron_sensor_quality_rejects_stale_observation() {
        let now = chrono::Utc::now();
        let latest = sensor_data_for_quality(&(now - chrono::Duration::seconds(301)).to_rfc3339());
        assert_eq!(cron_sensor_quality(&latest, now), CronInputQuality::Stale);
    }

    #[test]
    fn cron_sensor_quality_rejects_sensor_error_and_malformed_timestamp() {
        let now = chrono::Utc::now();
        let mut errored = sensor_data_for_quality(&now.to_rfc3339());
        errored.err_ph = Some(true);
        assert_eq!(
            cron_sensor_quality(&errored, now),
            CronInputQuality::Invalid
        );

        let malformed = sensor_data_for_quality("not-a-timestamp");
        assert_eq!(
            cron_sensor_quality(&malformed, now),
            CronInputQuality::Unknown
        );
    }

    #[test]
    fn computes_next_run_for_daily_7am_expression() {
        let now = chrono_tz::Asia::Ho_Chi_Minh
            .with_ymd_and_hms(2026, 9, 4, 8, 0, 0)
            .unwrap();
        let next = compute_next_run("0 0 7 * * * *", now).unwrap();
        assert_eq!(
            next.format("%Y-%m-%d %H:%M").to_string(),
            "2026-09-05 07:00"
        );
    }

    #[test]
    fn invalid_expression_returns_error() {
        assert!(
            compute_next_run(
                "not a cron",
                chrono::Utc::now().with_timezone(&chrono_tz::Asia::Ho_Chi_Minh)
            )
            .is_err()
        );
    }

    #[test]
    fn cron_schedule_from_ir_parses_valid_cron_trigger() {
        let ir = json!({
            "kind": "alert",
            "trigger": { "type": "cron", "cronExpression": "0 0 7 * * *", "timezone": "Asia/Ho_Chi_Minh" }
        });
        let schedule = cron_schedule_from_ir(&ir).expect("valid cron");
        match schedule {
            CronSchedule::Cron { expression, tz } => {
                assert_eq!(expression, "0 0 7 * * *");
                assert_eq!(tz.name(), "Asia/Ho_Chi_Minh");
            }
            CronSchedule::NotCron => panic!("expected Cron"),
        }
    }

    #[test]
    fn cron_schedule_from_ir_is_not_cron_for_other_triggers_or_missing_trigger() {
        for trigger in [
            json!({ "type": "sensor" }),
            json!({ "type": "fsm" }),
            json!({ "type": "webhook" }),
        ] {
            let ir = json!({ "kind": "alert", "trigger": trigger });
            assert_eq!(cron_schedule_from_ir(&ir).unwrap(), CronSchedule::NotCron);
        }
        let no_trigger = json!({ "kind": "alert" });
        assert_eq!(
            cron_schedule_from_ir(&no_trigger).unwrap(),
            CronSchedule::NotCron
        );
    }

    #[test]
    fn cron_schedule_from_ir_rejects_bad_expression_or_timezone() {
        let empty_expr = json!({
            "kind": "alert",
            "trigger": { "type": "cron", "cronExpression": "", "timezone": "Asia/Ho_Chi_Minh" }
        });
        assert!(cron_schedule_from_ir(&empty_expr).is_err());

        let bad_expr = json!({
            "kind": "alert",
            "trigger": { "type": "cron", "cronExpression": "not a cron", "timezone": "Asia/Ho_Chi_Minh" }
        });
        assert!(cron_schedule_from_ir(&bad_expr).is_err());

        let bad_tz = json!({
            "kind": "alert",
            "trigger": { "type": "cron", "cronExpression": "0 0 7 * * *", "timezone": "Not/AZone" }
        });
        assert!(cron_schedule_from_ir(&bad_tz).is_err());
    }

    #[test]
    fn validate_cron_trigger_accepts_none_or_non_cron_ir() {
        assert!(validate_cron_trigger(None).is_ok());
        assert!(validate_cron_trigger(Some(&json!({ "trigger": { "type": "sensor" } }))).is_ok());
    }

    #[test]
    fn validate_cron_trigger_rejects_invalid_expression() {
        let bad = json!({
            "kind": "alert",
            "trigger": { "type": "cron", "cronExpression": "61 0 7 * * *", "timezone": "Asia/Ho_Chi_Minh" }
        });
        assert!(validate_cron_trigger(Some(&bad)).is_err());
    }
}
