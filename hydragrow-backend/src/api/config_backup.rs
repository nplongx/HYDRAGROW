use actix_web::{HttpMessage, HttpRequest, HttpResponse, Responder, web};
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use serde_json::{Map, Value, json};
use sha2::{Digest, Sha256};
use sqlx::{PgConnection, Row};
use tracing::{error, info, warn};
use uuid::Uuid;

use crate::AppState;
use crate::api::device_pairing::require_device_owner;
use crate::api::middleware::auth::AuthContext;
use crate::db::postgres::{NewSystemEventRecord, insert_system_event};
use crate::metrics::BACKUP_RESTORE_TOTAL;
use crate::models::config::{
    DeviceConfig, DosingCalibration, SafetyConfig, SensorCalibration, WaterConfig, from_db_rows,
};

#[cfg(test)]
async fn ensure_config_backup_test_schema(pool: &sqlx::PgPool) {
    sqlx::migrate!()
        .run(pool)
        .await
        .expect("test database migrations must apply");
}

const BACKUP_FORMAT: &str = "hydragrow-backup";
const BACKUP_SCHEMA_VERSION: u8 = 1;
const CONFIG_TABLES: &[&str] = &[
    "device_config",
    "water_config",
    "safety_config",
    "sensor_calibration",
    "dosing_calibration",
];

const SECRET_KEYS: &[&str] = &[
    "password",
    "passwd",
    "api_key",
    "apikey",
    "bearer_token",
    "access_token",
    "privileged_control_token",
    "privileged_control_secret",
    "webhook_secret",
    "signing_key",
    "private_key",
    "mqtt_password",
    "mqtt_password_hash",
    "wifi_password",
    "firebase_token",
    "firebase_credentials",
    "session",
    "session_token",
    "cookie",
];

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "lowercase")]
pub enum BackupScope {
    Station,
    Device,
    Controller,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BackupProducer {
    pub application: String,
    pub version: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BackupSource {
    pub station_id: Option<String>,
    pub device_ids: Vec<String>,
    pub controller_ids: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BackupIntegrity {
    pub algorithm: String,
    pub digest: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BackupArtifact {
    pub format: String,
    pub schema_version: u8,
    pub created_at: DateTime<Utc>,
    pub producer: BackupProducer,
    pub scope: BackupScope,
    pub source: BackupSource,
    pub configuration: Value,
    pub integrity: BackupIntegrity,
}

#[derive(Debug, Serialize)]
struct PreviewResponse {
    status: &'static str,
    target_device_id: String,
    source_device_id: String,
    additions: Vec<String>,
    changes: Vec<String>,
    unchanged: Vec<String>,
    rejected: Vec<String>,
    warnings: Vec<String>,
    apply_permitted: bool,
}

fn auth_from(req: &HttpRequest) -> AuthContext {
    req.extensions()
        .get::<AuthContext>()
        .cloned()
        .unwrap_or_default()
}

fn require_backup_scope(req: &HttpRequest) -> Result<(), HttpResponse> {
    let auth = auth_from(req);
    let allowed = auth.has_scope("device:admin") || auth.has_scope("write:config");
    if allowed {
        Ok(())
    } else {
        Err(HttpResponse::Forbidden().json(json!({
            "error": "Missing required backup scope",
            "required_scope": "device:admin or write:config"
        })))
    }
}

fn contains_secret_key(value: &Value) -> Option<String> {
    match value {
        Value::Object(map) => {
            for (key, child) in map {
                if SECRET_KEYS
                    .iter()
                    .any(|secret| key.eq_ignore_ascii_case(secret))
                {
                    return Some(key.clone());
                }
                if let Some(found) = contains_secret_key(child) {
                    return Some(found);
                }
            }
            None
        }
        Value::Array(items) => items.iter().find_map(contains_secret_key),
        _ => None,
    }
}

fn redact_secrets(value: &Value) -> Value {
    match value {
        Value::Object(map) => Value::Object(
            map.iter()
                .filter(|(key, _)| {
                    !SECRET_KEYS
                        .iter()
                        .any(|secret| key.eq_ignore_ascii_case(secret))
                })
                .map(|(key, value)| (key.clone(), redact_secrets(value)))
                .collect(),
        ),
        Value::Array(items) => Value::Array(items.iter().map(redact_secrets).collect()),
        _ => value.clone(),
    }
}

/// Deterministic JSON representation: recursively sort object keys before hashing.
fn canonicalize(value: &Value) -> Value {
    match value {
        Value::Object(map) => {
            let mut sorted = Map::new();
            let mut entries: Vec<_> = map.iter().collect();
            entries.sort_by(|a, b| a.0.cmp(b.0));
            for (key, child) in entries {
                sorted.insert(key.clone(), canonicalize(child));
            }
            Value::Object(sorted)
        }
        Value::Array(items) => Value::Array(items.iter().map(canonicalize).collect()),
        _ => value.clone(),
    }
}

fn payload_without_digest(artifact: &BackupArtifact) -> Value {
    let mut value = serde_json::to_value(artifact).expect("BackupArtifact is serializable");
    if let Value::Object(root) = &mut value
        && let Some(Value::Object(integrity)) = root.get_mut("integrity")
    {
        integrity.remove("digest");
    }
    canonicalize(&value)
}

fn compute_digest(artifact: &BackupArtifact) -> String {
    let payload = serde_json::to_vec(&payload_without_digest(artifact))
        .expect("canonical backup payload is serializable");
    let mut hasher = Sha256::new();
    hasher.update(payload);
    hex::encode(hasher.finalize())
}

fn verify_integrity(artifact: &BackupArtifact) -> Result<(), String> {
    if artifact.integrity.algorithm != "SHA-256" {
        return Err("Unsupported integrity algorithm".to_string());
    }
    let expected = compute_digest(artifact);
    if artifact.integrity.digest != expected {
        return Err("Backup integrity digest mismatch".to_string());
    }
    Ok(())
}

fn validate_artifact(
    artifact: &BackupArtifact,
    target_device_id: &str,
) -> Result<String, Vec<String>> {
    let mut errors = Vec::new();
    if artifact.format != BACKUP_FORMAT {
        errors.push("Unsupported backup format".to_string());
    }
    if artifact.schema_version != BACKUP_SCHEMA_VERSION {
        errors.push(format!(
            "Unsupported backup schema_version: {}",
            artifact.schema_version
        ));
    }
    if artifact.producer.application != "HydraGrow" || artifact.producer.version.trim().is_empty() {
        errors.push("Invalid backup producer metadata".to_string());
    }
    if artifact.scope != BackupScope::Device {
        errors.push("Unsupported backup scope".to_string());
    }
    if artifact.source.device_ids.len() != 1 {
        errors.push("Device backup must contain exactly one source device_id".to_string());
    }
    let source_device_id = artifact
        .source
        .device_ids
        .first()
        .cloned()
        .unwrap_or_default();
    if source_device_id.is_empty() {
        errors.push("Missing source device_id".to_string());
    }
    if !artifact.source.controller_ids.is_empty() || artifact.source.station_id.is_some() {
        errors
            .push("Device backup cannot contain station/controller ownership metadata".to_string());
    }
    if let Some(secret) = contains_secret_key(&artifact.configuration) {
        errors.push(format!("Backup contains forbidden secret field: {secret}"));
    }
    if let Err(error) = verify_integrity(artifact) {
        errors.push(error);
    }

    let Some(configuration) = artifact.configuration.as_object() else {
        errors.push("configuration must be a JSON object".to_string());
        return Err(errors);
    };
    for table in CONFIG_TABLES {
        match configuration.get(*table) {
            Some(Value::Object(row)) if !row.is_empty() => {
                let mut allowed = table_columns(table)
                    .split(", ")
                    .map(str::to_string)
                    .collect::<std::collections::BTreeSet<_>>();
                allowed.insert("device_id".to_string());
                for key in row.keys() {
                    if !allowed.contains(key) {
                        errors.push(format!("Unsupported field in {table}: {key}"));
                    }
                }
                for key in &allowed {
                    if !row.contains_key(key) {
                        errors.push(format!("Missing field in {table}: {key}"));
                    }
                }
            }
            _ => errors.push(format!("Missing required configuration domain: {table}")),
        }
    }
    if let Some(Value::Object(row)) = configuration.get("device_config")
        && row.get("device_id").and_then(Value::as_str) != Some(source_device_id.as_str())
    {
        errors.push("device_config.device_id does not match source.device_ids[0]".to_string());
    }
    if target_device_id.trim().is_empty() {
        errors.push("Target device_id is empty".to_string());
    }

    if let Some(recipe) = configuration.get("recipe") {
        validate_recipe_shape(recipe, &mut errors);
    }

    if errors.is_empty() {
        Ok(source_device_id)
    } else {
        Err(errors)
    }
}

fn validate_recipe_shape(recipe: &Value, errors: &mut Vec<String>) {
    let Some(map) = recipe.as_object() else {
        errors.push("recipe must be a JSON object".to_string());
        return;
    };
    let allowed = ["id", "name", "crop", "description", "created_at", "stages"];
    for key in map.keys() {
        if !allowed.contains(&key.as_str()) {
            errors.push(format!("Unsupported field in recipe: {key}"));
        }
    }
    for key in ["id", "name", "crop", "created_at", "stages"] {
        if !map.contains_key(key) {
            errors.push(format!("Missing field in recipe: {key}"));
        }
    }
    if let Some(stages) = map.get("stages").and_then(Value::as_array) {
        let stage_allowed = [
            "id",
            "recipe_id",
            "stage_order",
            "name",
            "duration_days",
            "ec_target",
            "ec_tolerance",
            "ph_target",
            "ph_tolerance",
            "nutrient_a_ratio",
            "nutrient_b_ratio",
            "water_level_target",
            "water_change_interval_days",
            "water_change_drain_cm",
            "auto_dilute_ec_trigger",
            "misting_on_duration_ms",
            "misting_off_duration_ms",
            "max_dose_per_cycle_ml",
            "light_hours",
        ];
        for (index, stage) in stages.iter().enumerate() {
            let Some(stage_map) = stage.as_object() else {
                errors.push(format!("recipe.stages[{index}] must be an object"));
                continue;
            };
            for key in stage_map.keys() {
                if !stage_allowed.contains(&key.as_str()) {
                    errors.push(format!(
                        "Unsupported field in recipe.stages[{index}]: {key}"
                    ));
                }
            }
            for key in [
                "id",
                "recipe_id",
                "stage_order",
                "name",
                "duration_days",
                "ec_target",
                "ec_tolerance",
                "ph_target",
                "ph_tolerance",
                "nutrient_a_ratio",
                "nutrient_b_ratio",
                "water_level_target",
                "misting_on_duration_ms",
                "misting_off_duration_ms",
            ] {
                if !stage_map.contains_key(key) {
                    errors.push(format!("Missing field in recipe.stages[{index}]: {key}"));
                }
            }
        }
    }
}

async fn fetch_row_json(
    conn: &mut PgConnection,
    table: &str,
    device_id: &str,
) -> Result<Option<Value>, sqlx::Error> {
    let query =
        format!("SELECT row_to_json({table}) AS row_json FROM {table} WHERE device_id = $1");
    let row = sqlx::query(&query)
        .bind(device_id)
        .fetch_optional(&mut *conn)
        .await?;
    row.map(|row| row.try_get("row_json")).transpose()
}

async fn fetch_recipe_json(
    conn: &mut PgConnection,
    recipe_id: &str,
) -> Result<Option<Value>, sqlx::Error> {
    let recipe =
        sqlx::query("SELECT row_to_json(crop_recipes) AS row_json FROM crop_recipes WHERE id = $1")
            .bind(recipe_id)
            .fetch_optional(&mut *conn)
            .await?;
    let Some(recipe) = recipe else {
        return Ok(None);
    };
    let mut value: Value = recipe.try_get("row_json")?;
    let stages = sqlx::query(
        "SELECT row_to_json(crop_recipe_stages) AS row_json FROM crop_recipe_stages WHERE recipe_id = $1 ORDER BY stage_order",
    )
    .bind(recipe_id)
    .fetch_all(&mut *conn)
    .await?;
    let stage_values: Vec<Value> = stages
        .into_iter()
        .map(|row| row.try_get("row_json"))
        .collect::<Result<_, sqlx::Error>>()?;
    if let Value::Object(map) = &mut value {
        map.insert("stages".to_string(), Value::Array(stage_values));
    }
    Ok(Some(value))
}

async fn fetch_export_configuration(
    conn: &mut PgConnection,
    device_id: &str,
) -> Result<Value, String> {
    let mut config = Map::new();
    for table in CONFIG_TABLES {
        let row = fetch_row_json(conn, table, device_id)
            .await
            .map_err(|e| format!("Failed to read {table}: {e}"))?
            .ok_or_else(|| format!("Required configuration domain not found: {table}"))?;
        config.insert((*table).to_string(), redact_secrets(&row));
    }

    let active_recipe =
        sqlx::query("SELECT recipe_id FROM device_active_recipes WHERE device_id = $1")
            .bind(device_id)
            .fetch_optional(&mut *conn)
            .await
            .map_err(|e| format!("Failed to read active recipe: {e}"))?;
    if let Some(row) = active_recipe {
        let recipe_id: String = row.try_get("recipe_id").map_err(|e| e.to_string())?;
        if let Some(recipe) = fetch_recipe_json(conn, &recipe_id)
            .await
            .map_err(|e| e.to_string())?
        {
            // The active assignment/current stage is runtime state; export the immutable recipe template only.
            config.insert("recipe".to_string(), redact_secrets(&recipe));
        }
    }
    Ok(Value::Object(config))
}

fn build_artifact(configuration: Value, source_device_id: String) -> BackupArtifact {
    let mut artifact = BackupArtifact {
        format: BACKUP_FORMAT.to_string(),
        schema_version: BACKUP_SCHEMA_VERSION,
        created_at: Utc::now(),
        producer: BackupProducer {
            application: "HydraGrow".to_string(),
            version: env!("CARGO_PKG_VERSION").to_string(),
        },
        scope: BackupScope::Device,
        source: BackupSource {
            station_id: None,
            device_ids: vec![source_device_id],
            controller_ids: vec![],
        },
        configuration,
        integrity: BackupIntegrity {
            algorithm: "SHA-256".to_string(),
            digest: String::new(),
        },
    };
    artifact.integrity.digest = compute_digest(&artifact);
    artifact
}

fn mapped_domain(
    configuration: &Value,
    table: &str,
    source_device_id: &str,
    target_device_id: &str,
) -> Result<Value, String> {
    let row = configuration
        .get(table)
        .and_then(Value::as_object)
        .ok_or_else(|| format!("Missing configuration domain: {table}"))?;
    let mut mapped = Value::Object(row.clone());
    if let Value::Object(map) = &mut mapped {
        let Some(id) = map.get("device_id").and_then(Value::as_str) else {
            return Err(format!("{table}.device_id missing"));
        };
        if id != source_device_id {
            return Err(format!("{table}.device_id does not match source device"));
        }
        map.insert(
            "device_id".to_string(),
            Value::String(target_device_id.to_string()),
        );
    }
    Ok(mapped)
}

async fn preview_restore(
    conn: &mut PgConnection,
    configuration: &Value,
    source_device_id: &str,
    target_device_id: &str,
) -> Result<(Vec<String>, Vec<String>, Vec<String>), String> {
    let mut additions = Vec::new();
    let mut changes = Vec::new();
    let mut unchanged = Vec::new();
    for table in CONFIG_TABLES {
        let desired = canonicalize(&mapped_domain(
            configuration,
            table,
            source_device_id,
            target_device_id,
        )?);
        let current = fetch_row_json(conn, table, target_device_id)
            .await
            .map_err(|e| e.to_string())?;
        match current {
            None => additions.push((*table).to_string()),
            Some(value) if canonicalize(&value) == desired => unchanged.push((*table).to_string()),
            Some(_) => changes.push((*table).to_string()),
        }
    }
    Ok((additions, changes, unchanged))
}

async fn apply_domain(conn: &mut PgConnection, table: &str, mapped: &Value) -> Result<(), String> {
    let query = format!(
        "INSERT INTO {table} SELECT * FROM jsonb_populate_record(NULL::{table}, $1::jsonb) ON CONFLICT (device_id) DO UPDATE SET ({cols}) = (SELECT {cols} FROM jsonb_populate_record(NULL::{table}, $1::jsonb))",
        cols = table_columns(table)
    );
    sqlx::query(&query)
        .bind(mapped)
        .execute(&mut *conn)
        .await
        .map(|_| ())
        .map_err(|e| format!("Failed to apply {table}: {e}"))
}

fn table_columns(table: &str) -> &'static str {
    match table {
        "device_config" => {
            "ec_target, ec_tolerance, ph_target, ph_tolerance, control_mode, is_enabled, delay_between_a_and_b_sec, last_updated"
        }
        "water_config" => {
            "water_level_min, water_level_target, water_level_max, water_level_drain, water_level_tolerance, auto_refill_enabled, auto_drain_overflow, auto_dilute_enabled, dilute_drain_amount_cm, scheduled_water_change_enabled, scheduled_drain_amount_cm, misting_on_duration_ms, misting_off_duration_ms, last_updated, tank_height, water_change_cron, misting_temp_threshold, high_temp_misting_on_duration_ms, high_temp_misting_off_duration_ms"
        }
        "safety_config" => {
            "min_ec_limit, max_ec_limit, min_ph_limit, max_ph_limit, max_ec_delta, max_ph_delta, max_dose_per_cycle, max_dose_per_hour, cooldown_sec, min_temp_limit, max_temp_limit, water_level_critical_min, max_refill_cycles_per_hour, max_drain_cycles_per_hour, max_refill_duration_sec, max_drain_duration_sec, emergency_shutdown, ec_ack_threshold, ph_ack_threshold, water_ack_threshold, last_updated"
        }
        "sensor_calibration" => {
            "ph_v7, ph_v4, ec_factor, ec_offset, temp_offset, temp_compensation_beta, publish_interval, moving_average_window, enable_ph_sensor, enable_ec_sensor, enable_temp_sensor, enable_water_level_sensor, last_calibrated, ph_v10, ph_calibration_mode"
        }
        "dosing_calibration" => {
            "ec_gain_per_ml, ph_shift_up_per_ml, ph_shift_down_per_ml, active_mixing_sec, sensor_stabilize_sec, ec_step_ratio, ph_step_ratio, pump_a_capacity_ml_per_sec, soft_start_duration, scheduled_mixing_interval_sec, scheduled_mixing_duration_sec, dosing_pwm_percent, osaka_mixing_pwm_percent, osaka_misting_pwm_percent, last_calibrated, pump_b_capacity_ml_per_sec, pump_ph_up_capacity_ml_per_sec, pump_ph_down_capacity_ml_per_sec, dosing_min_pwm_percent, pump_a_min_pwm_percent, pump_b_min_pwm_percent, pump_ph_up_min_pwm_percent, pump_ph_down_min_pwm_percent, dosing_pulse_on_ms, dosing_pulse_off_ms, dosing_min_dose_ml, dosing_max_pulse_count_per_cycle, step_ratio_ec, step_ratio_ph, ec_a_step_ratio, ec_b_step_ratio, ph_up_step_ratio, ph_down_step_ratio, best_ec_a_ratio, best_ec_b_ratio, best_ph_up_ratio, best_ph_down_ratio, interaction_matrix, matrix_update_count, matrix_is_warm, best_ec_ratio, best_ph_ratio, tuner_state, kalman_confidence, adaptive_mixing_sec, adaptive_stabilize_sec, effective_ec_tolerance, effective_ph_tolerance"
        }
        _ => unreachable!("unsupported backup table"),
    }
}

async fn apply_recipe(conn: &mut PgConnection, recipe: &Value) -> Result<(), String> {
    let recipe_map = recipe
        .as_object()
        .ok_or_else(|| "recipe must be an object".to_string())?;
    let recipe_id = recipe_map
        .get("id")
        .and_then(Value::as_str)
        .ok_or_else(|| "recipe.id missing".to_string())?;
    let stages = recipe_map
        .get("stages")
        .and_then(Value::as_array)
        .ok_or_else(|| "recipe.stages missing".to_string())?;

    let existing =
        sqlx::query("SELECT row_to_json(crop_recipes) AS row_json FROM crop_recipes WHERE id = $1")
            .bind(recipe_id)
            .fetch_optional(&mut *conn)
            .await
            .map_err(|e| e.to_string())?;
    if let Some(existing) = existing {
        let existing_value: Value = existing.try_get("row_json").map_err(|e| e.to_string())?;
        let mut desired = recipe.clone();
        if let Value::Object(map) = &mut desired {
            map.remove("stages");
        }
        if canonicalize(&desired) != canonicalize(&existing_value) {
            return Err(format!("Immutable recipe identity conflict: {recipe_id}"));
        }

        let existing_stages = sqlx::query(
            "SELECT row_to_json(crop_recipe_stages) AS row_json FROM crop_recipe_stages WHERE recipe_id = $1 ORDER BY stage_order",
        )
        .bind(recipe_id)
        .fetch_all(&mut *conn)
        .await
        .map_err(|e| e.to_string())?;
        let current_stages: Vec<Value> = existing_stages
            .into_iter()
            .map(|row| row.try_get("row_json"))
            .collect::<Result<_, sqlx::Error>>()
            .map_err(|e| e.to_string())?;
        let desired_stages = stages.clone();
        if canonicalize(&Value::Array(desired_stages))
            != canonicalize(&Value::Array(current_stages))
        {
            return Err(format!(
                "Immutable recipe stage identity conflict: {recipe_id}"
            ));
        }
    } else {
        let mut base = recipe.clone();
        if let Value::Object(map) = &mut base {
            map.remove("stages");
        }
        sqlx::query("INSERT INTO crop_recipes SELECT * FROM jsonb_populate_record(NULL::crop_recipes, $1::jsonb)")
            .bind(base)
            .execute(&mut *conn)
            .await
            .map_err(|e| format!("Failed to create recipe: {e}"))?;

        for (index, stage) in stages.iter().enumerate() {
            let mut stage_value = stage.clone();
            if let Value::Object(map) = &mut stage_value {
                map.entry("id")
                    .or_insert_with(|| Value::String(Uuid::new_v4().to_string()));
                map.insert(
                    "recipe_id".to_string(),
                    Value::String(recipe_id.to_string()),
                );
                map.insert("stage_order".to_string(), Value::from((index + 1) as i32));
            }
            sqlx::query("INSERT INTO crop_recipe_stages SELECT * FROM jsonb_populate_record(NULL::crop_recipe_stages, $1::jsonb)")
                .bind(stage_value)
                .execute(&mut *conn)
                .await
                .map_err(|e| format!("Failed to create recipe stage: {e}"))?;
        }
    }
    Ok(())
}

async fn persist_restore_sync(
    tx: &mut sqlx::Transaction<'_, sqlx::Postgres>,
    configuration: &Value,
    source_device_id: &str,
    target_device_id: &str,
) -> Result<i64, String> {
    let mapped =
        |table: &str| mapped_domain(configuration, table, source_device_id, target_device_id);

    let device: DeviceConfig = serde_json::from_value(mapped("device_config")?)
        .map_err(|e| format!("Invalid restored device_config: {e}"))?;
    let water: WaterConfig = serde_json::from_value(mapped("water_config")?)
        .map_err(|e| format!("Invalid restored water_config: {e}"))?;
    let safety: SafetyConfig = serde_json::from_value(mapped("safety_config")?)
        .map_err(|e| format!("Invalid restored safety_config: {e}"))?;
    let dosing: DosingCalibration = serde_json::from_value(mapped("dosing_calibration")?)
        .map_err(|e| format!("Invalid restored dosing_calibration: {e}"))?;
    let sensor: SensorCalibration = serde_json::from_value(mapped("sensor_calibration")?)
        .map_err(|e| format!("Invalid restored sensor_calibration: {e}"))?;

    let version = crate::db::config_sync::next_version(tx, target_device_id)
        .await
        .map_err(|e| format!("Failed to allocate restore config version: {e}"))?;

    let mut desired_controller =
        serde_json::to_value(from_db_rows(&device, &water, &safety, &dosing, &sensor))
            .map_err(|e| format!("Failed to serialize restored controller config: {e}"))?;
    let mut desired_sensor = serde_json::to_value(&sensor)
        .map_err(|e| format!("Failed to serialize restored sensor config: {e}"))?;
    desired_controller["config_version"] = json!(version);
    desired_sensor["config_version"] = json!(version);

    crate::db::config_sync::upsert_desired(
        &mut **tx,
        target_device_id,
        version,
        &desired_controller,
        &desired_sensor,
    )
    .await
    .map_err(|e| format!("Failed to persist restore configuration sync: {e}"))?;

    Ok(version)
}

pub async fn export_backup(
    path: web::Path<String>,
    req: HttpRequest,
    app_state: web::Data<AppState>,
) -> impl Responder {
    if let Err(resp) = require_backup_scope(&req) {
        return resp;
    }
    let device_id = path.into_inner();
    if let Err(resp) = require_device_owner(&req, &app_state, &device_id).await {
        return resp;
    }

    let mut tx = match app_state.pg_pool.begin().await {
        Ok(tx) => tx,
        Err(e) => {
            return HttpResponse::InternalServerError()
                .json(json!({"error": format!("DB transaction failed: {e}")}));
        }
    };
    let configuration = match fetch_export_configuration(&mut tx, &device_id).await {
        Ok(value) => value,
        Err(error) => {
            warn!(%device_id, %error, "Backup export failed");
            return HttpResponse::InternalServerError().json(json!({"error": error}));
        }
    };
    if let Err(e) = tx.rollback().await {
        warn!(%device_id, ?e, "Backup read transaction rollback failed");
    }

    let artifact = build_artifact(configuration, device_id.clone());
    BACKUP_RESTORE_TOTAL
        .with_label_values(&["backup", "created"])
        .inc();
    info!(%device_id, schema_version = BACKUP_SCHEMA_VERSION, "Exported canonical HydraGrow backup");
    HttpResponse::Ok()
        .content_type("application/json")
        .insert_header((
            "Content-Disposition",
            format!("attachment; filename=\"hydragrow_backup_{device_id}.json\""),
        ))
        .json(artifact)
}

async fn validate_common(
    req: &HttpRequest,
    app_state: &web::Data<AppState>,
    device_id: &str,
    artifact: &BackupArtifact,
) -> Result<String, HttpResponse> {
    require_backup_scope(req)?;
    require_device_owner(req, app_state, device_id).await?;
    validate_artifact(artifact, device_id).map_err(|errors| {
        BACKUP_RESTORE_TOTAL
            .with_label_values(&["restore", "rejected"])
            .inc();
        HttpResponse::BadRequest().json(json!({"status": "rejected", "errors": errors}))
    })
}

pub async fn validate_backup(
    path: web::Path<String>,
    req: HttpRequest,
    body: web::Json<BackupArtifact>,
    app_state: web::Data<AppState>,
) -> impl Responder {
    let device_id = path.into_inner();
    let artifact = body.into_inner();
    let source = match validate_common(&req, &app_state, &device_id, &artifact).await {
        Ok(source) => source,
        Err(resp) => return resp,
    };
    let mut conn = match app_state.pg_pool.acquire().await {
        Ok(conn) => conn,
        Err(e) => {
            return HttpResponse::InternalServerError()
                .json(json!({"error": format!("DB connection failed: {e}")}));
        }
    };
    let (additions, changes, unchanged) =
        match preview_restore(&mut conn, &artifact.configuration, &source, &device_id).await {
            Ok(value) => value,
            Err(e) => {
                return HttpResponse::BadRequest()
                    .json(json!({"status": "rejected", "errors": [e]}));
            }
        };
    let mut warnings = Vec::new();
    if source != device_id {
        warnings.push("Source device differs from explicit target; device_id fields will be mapped to target.".to_string());
    }
    if artifact.configuration.get("recipe").is_some() {
        warnings.push(
            "Recipe template is restored; active recipe/current stage are not restored."
                .to_string(),
        );
    }
    HttpResponse::Ok().json(PreviewResponse {
        status: "validated",
        target_device_id: device_id,
        source_device_id: source,
        additions,
        changes,
        unchanged,
        rejected: vec![],
        warnings,
        apply_permitted: true,
    })
}

pub async fn import_backup(
    path: web::Path<String>,
    req: HttpRequest,
    body: web::Json<BackupArtifact>,
    app_state: web::Data<AppState>,
) -> impl Responder {
    let target_device_id = path.into_inner();
    let artifact = body.into_inner();
    let source_device_id =
        match validate_common(&req, &app_state, &target_device_id, &artifact).await {
            Ok(source) => source,
            Err(resp) => return resp,
        };

    let mut tx = match app_state.pg_pool.begin().await {
        Ok(tx) => tx,
        Err(e) => {
            error!(%target_device_id, ?e, "Restore transaction begin failed");
            return HttpResponse::InternalServerError()
                .json(json!({"status": "rejected", "error": "db_transaction_begin_failed"}));
        }
    };

    for table in CONFIG_TABLES {
        let mapped = match mapped_domain(
            &artifact.configuration,
            table,
            &source_device_id,
            &target_device_id,
        ) {
            Ok(value) => value,
            Err(e) => {
                let _ = tx.rollback().await;
                return HttpResponse::BadRequest().json(json!({"status":"rejected","error":e}));
            }
        };
        if let Err(e) = apply_domain(&mut tx, table, &mapped).await {
            error!(%target_device_id, table, error = %e, "Restore transaction failed; rolling back");
            let _ = tx.rollback().await;
            BACKUP_RESTORE_TOTAL
                .with_label_values(&["restore", "rejected"])
                .inc();
            return HttpResponse::InternalServerError()
                .json(json!({"status": "rejected", "error": "db_transaction_failed"}));
        }
    }

    if let Some(recipe) = artifact.configuration.get("recipe")
        && let Err(e) = apply_recipe(&mut tx, recipe).await
    {
        error!(%target_device_id, error = %e, "Recipe restore failed; rolling back");
        let _ = tx.rollback().await;
        BACKUP_RESTORE_TOTAL
            .with_label_values(&["restore", "rejected"])
            .inc();
        return HttpResponse::Conflict().json(json!({"status": "rejected", "error": e}));
    }

    let config_version = match persist_restore_sync(
        &mut tx,
        &artifact.configuration,
        &source_device_id,
        &target_device_id,
    )
    .await
    {
        Ok(version) => version,
        Err(e) => {
            error!(%target_device_id, error = %e, "Restore ConfigurationSync persistence failed; rolling back");
            let _ = tx.rollback().await;
            BACKUP_RESTORE_TOTAL
                .with_label_values(&["restore", "rejected"])
                .inc();
            return HttpResponse::InternalServerError()
                .json(json!({"status": "rejected", "error": "config_sync_persistence_failed"}));
        }
    };

    if let Err(e) = tx.commit().await {
        error!(%target_device_id, ?e, "Restore transaction commit failed");
        BACKUP_RESTORE_TOTAL
            .with_label_values(&["restore", "rejected"])
            .inc();
        return HttpResponse::InternalServerError()
            .json(json!({"status": "rejected", "error": "db_commit_failed"}));
    }

    // Audit is metadata-only; it deliberately does not persist the backup payload.
    let audit = NewSystemEventRecord {
        device_id: target_device_id.clone(),
        level: "info".to_string(),
        category: "user_action".to_string(),
        title: "Khôi phục cấu hình từ backup".to_string(),
        message: format!("Đã commit backup cấu hình cho thiết bị {target_device_id}."),
        reason: Some("backup_restore".to_string()),
        metadata: Some(json!({
            "event_type": "backup_restore",
            "schema_version": artifact.schema_version,
            "source_device_id": source_device_id,
            "target_device_id": target_device_id,
            "digest": artifact.integrity.digest,
        })),
        timestamp: Utc::now().timestamp_millis(),
        source: "api".to_string(),
        primary_reason_code: Some("backup_restore_committed".to_string()),
    };
    if let Err(e) = insert_system_event(&app_state.pg_pool, &audit).await {
        warn!(%target_device_id, ?e, "Restore committed but audit event could not be persisted");
    }

    BACKUP_RESTORE_TOTAL
        .with_label_values(&["restore", "applied_sync_pending"])
        .inc();
    HttpResponse::Ok().json(json!({
        "status": "applied_sync_pending",
        "device_id": target_device_id,
        "source_device_id": source_device_id,
        "config_version": config_version,
        "remote_confirmation": "not_confirmed"
    }))
}

pub fn init_routes(cfg: &mut web::ServiceConfig) {
    cfg.route("/backup", web::get().to(export_backup))
        .route("/restore/validate", web::post().to(validate_backup))
        .route("/restore", web::post().to(import_backup));
}

#[cfg(test)]
#[allow(clippy::unwrap_used, clippy::explicit_auto_deref)]
mod tests {
    use super::*;

    fn fixture() -> BackupArtifact {
        build_artifact(
            json!({
                "device_config": {"device_id":"dev-a","ec_target":1.4},
                "water_config": {"device_id":"dev-a","water_level_target":20},
                "safety_config": {"device_id":"dev-a","max_ec_limit":3},
                "sensor_calibration": {"device_id":"dev-a","ec_factor":880},
                "dosing_calibration": {"device_id":"dev-a","dosing_pwm_percent":50}
            }),
            "dev-a".to_string(),
        )
    }

    #[test]
    fn digest_is_deterministic() {
        let artifact = fixture();
        assert_eq!(artifact.integrity.digest, compute_digest(&artifact));
        assert!(verify_integrity(&artifact).is_ok());
    }

    #[test]
    fn digest_changes_when_configuration_changes() {
        let mut artifact = fixture();
        let original = artifact.integrity.digest.clone();
        artifact.configuration["device_config"]["ec_target"] = json!(1.5);
        assert_ne!(original, compute_digest(&artifact));
        assert!(verify_integrity(&artifact).is_err());
    }

    #[test]
    fn import_rejects_secrets() {
        let mut artifact = fixture();
        artifact.configuration["device_config"]["password"] = json!("secret");
        artifact.integrity.digest = compute_digest(&artifact);
        let errors = validate_artifact(&artifact, "dev-a").unwrap_err();
        assert!(errors.iter().any(|e| e.contains("forbidden secret field")));
    }

    #[test]
    fn source_device_can_be_mapped_to_explicit_target() {
        let artifact = fixture();
        let mapped =
            mapped_domain(&artifact.configuration, "device_config", "dev-a", "dev-b").unwrap();
        assert_eq!(mapped["device_id"], "dev-b");
    }

    #[tokio::test]
    async fn postgres_restore_transaction_rolls_back_prior_domain_on_failure() {
        let _ = dotenvy::dotenv();
        let Ok(database_url) = std::env::var("DATABASE_URL") else {
            return;
        };
        let pool = sqlx::PgPool::connect(&database_url).await.unwrap();
        ensure_config_backup_test_schema(&pool).await;
        let source_id: Option<String> =
            sqlx::query_scalar("SELECT device_id FROM device_config ORDER BY device_id LIMIT 1")
                .fetch_optional(&pool)
                .await
                .unwrap();
        let Some(source_id) = source_id else {
            return;
        };

        let temp_id = format!("p1-5-tx-{}", Uuid::new_v4());
        let mut setup = pool.begin().await.unwrap();
        for table in ["device_config", "water_config"] {
            let mut row = fetch_row_json(&mut *setup, table, &source_id)
                .await
                .unwrap()
                .expect("source config exists");
            row["device_id"] = Value::String(temp_id.clone());
            apply_domain(&mut *setup, table, &row).await.unwrap();
        }
        setup.commit().await.unwrap();

        let original: f32 =
            sqlx::query_scalar("SELECT ec_target FROM device_config WHERE device_id = $1")
                .bind(&temp_id)
                .fetch_one(&pool)
                .await
                .unwrap();

        let mut tx = pool.begin().await.unwrap();
        let mut device = fetch_row_json(&mut *tx, "device_config", &temp_id)
            .await
            .unwrap()
            .unwrap();
        device["ec_target"] = json!(original + 0.1);
        apply_domain(&mut *tx, "device_config", &device)
            .await
            .unwrap();

        let mut invalid_water = fetch_row_json(&mut *tx, "water_config", &temp_id)
            .await
            .unwrap()
            .unwrap();
        invalid_water["water_level_target"] = json!("not-a-number");
        assert!(
            apply_domain(&mut *tx, "water_config", &invalid_water)
                .await
                .is_err()
        );
        tx.rollback().await.unwrap();

        let after: f32 =
            sqlx::query_scalar("SELECT ec_target FROM device_config WHERE device_id = $1")
                .bind(&temp_id)
                .fetch_one(&pool)
                .await
                .unwrap();
        assert_eq!(after, original);

        sqlx::query("DELETE FROM device_config WHERE device_id = $1")
            .bind(&temp_id)
            .execute(&pool)
            .await
            .unwrap();
    }

    #[tokio::test]
    async fn postgres_export_preview_and_atomic_apply_roundtrip() {
        let _ = dotenvy::dotenv();
        let Ok(database_url) = std::env::var("DATABASE_URL") else {
            return;
        };
        let pool = sqlx::PgPool::connect(&database_url).await.unwrap();
        ensure_config_backup_test_schema(&pool).await;
        let source_id: Option<String> = sqlx::query_scalar(
            "SELECT d.device_id FROM device_config d JOIN water_config w USING(device_id) JOIN safety_config s USING(device_id) JOIN sensor_calibration se USING(device_id) JOIN dosing_calibration dc USING(device_id) ORDER BY d.device_id LIMIT 1",
        )
        .fetch_optional(&pool)
        .await
        .unwrap();
        let Some(source_id) = source_id else {
            return;
        };

        let mut export_conn = pool.acquire().await.unwrap();
        let configuration = fetch_export_configuration(&mut export_conn, &source_id)
            .await
            .unwrap();
        assert!(contains_secret_key(&configuration).is_none());
        let artifact = build_artifact(configuration, source_id.clone());
        assert!(validate_artifact(&artifact, &source_id).is_ok());
        let (additions, changes, unchanged) = preview_restore(
            &mut export_conn,
            &artifact.configuration,
            &source_id,
            &source_id,
        )
        .await
        .unwrap();
        assert!(additions.is_empty());
        assert!(changes.is_empty());
        assert_eq!(unchanged.len(), CONFIG_TABLES.len());

        let temp_id = format!("p1-5-roundtrip-{}", Uuid::new_v4());
        let mut setup = pool.begin().await.unwrap();
        for table in CONFIG_TABLES {
            let mut row = fetch_row_json(&mut *setup, table, &source_id)
                .await
                .unwrap()
                .expect("source config exists");
            row["device_id"] = Value::String(temp_id.clone());
            apply_domain(&mut *setup, table, &row).await.unwrap();
        }
        setup.commit().await.unwrap();

        let mut mapped_artifact = artifact.clone();
        mapped_artifact.integrity.digest.clear();
        mapped_artifact.source.device_ids = vec![source_id.clone()];
        mapped_artifact.configuration["device_config"]["ec_target"] = json!(2.1);
        mapped_artifact.integrity.digest = compute_digest(&mapped_artifact);
        assert!(validate_artifact(&mapped_artifact, &temp_id).is_ok());

        let mut tx = pool.begin().await.unwrap();
        for table in CONFIG_TABLES {
            let mapped =
                mapped_domain(&mapped_artifact.configuration, table, &source_id, &temp_id).unwrap();
            apply_domain(&mut *tx, table, &mapped).await.unwrap();
            apply_domain(&mut *tx, table, &mapped).await.unwrap();
        }
        tx.commit().await.unwrap();

        let applied: f32 =
            sqlx::query_scalar("SELECT ec_target FROM device_config WHERE device_id = $1")
                .bind(&temp_id)
                .fetch_one(&pool)
                .await
                .unwrap();
        assert_eq!(applied, 2.1);
        let count: i64 =
            sqlx::query_scalar("SELECT COUNT(*) FROM device_config WHERE device_id = $1")
                .bind(&temp_id)
                .fetch_one(&pool)
                .await
                .unwrap();
        assert_eq!(count, 1);

        sqlx::query("DELETE FROM device_config WHERE device_id = $1")
            .bind(&temp_id)
            .execute(&pool)
            .await
            .unwrap();
    }

    #[tokio::test]
    async fn postgres_restore_persists_sync_revision_in_same_transaction() {
        let _ = dotenvy::dotenv();
        let Ok(database_url) = std::env::var("DATABASE_URL") else {
            return;
        };
        let pool = sqlx::PgPool::connect(&database_url).await.unwrap();
        ensure_config_backup_test_schema(&pool).await;

        let source_id: Option<String> = sqlx::query_scalar(
            "SELECT d.device_id FROM device_config d JOIN water_config w USING(device_id) JOIN safety_config s USING(device_id) JOIN sensor_calibration se USING(device_id) JOIN dosing_calibration dc USING(device_id) ORDER BY d.device_id LIMIT 1",
        )
        .fetch_optional(&pool)
        .await
        .unwrap();
        let Some(source_id) = source_id else {
            return;
        };

        let mut export_conn = pool.acquire().await.unwrap();
        let configuration = fetch_export_configuration(&mut export_conn, &source_id)
            .await
            .unwrap();
        let artifact = build_artifact(configuration, source_id.clone());
        let target_id = format!("p1-5-sync-{}", Uuid::new_v4());

        let mut tx = pool.begin().await.unwrap();
        for table in CONFIG_TABLES {
            let mapped =
                mapped_domain(&artifact.configuration, table, &source_id, &target_id).unwrap();
            apply_domain(&mut *tx, table, &mapped).await.unwrap();
        }
        let version =
            persist_restore_sync(&mut tx, &artifact.configuration, &source_id, &target_id)
                .await
                .unwrap();
        tx.commit().await.unwrap();

        let sync = crate::db::config_sync::get(&pool, &target_id)
            .await
            .unwrap()
            .expect("restore sync row must be durable");
        let mapped_device: DeviceConfig = serde_json::from_value(
            mapped_domain(
                &artifact.configuration,
                "device_config",
                &source_id,
                &target_id,
            )
            .unwrap(),
        )
        .unwrap();
        let mapped_water: WaterConfig = serde_json::from_value(
            mapped_domain(
                &artifact.configuration,
                "water_config",
                &source_id,
                &target_id,
            )
            .unwrap(),
        )
        .unwrap();
        let mapped_safety: SafetyConfig = serde_json::from_value(
            mapped_domain(
                &artifact.configuration,
                "safety_config",
                &source_id,
                &target_id,
            )
            .unwrap(),
        )
        .unwrap();
        let mapped_dosing: DosingCalibration = serde_json::from_value(
            mapped_domain(
                &artifact.configuration,
                "dosing_calibration",
                &source_id,
                &target_id,
            )
            .unwrap(),
        )
        .unwrap();
        let mapped_sensor: SensorCalibration = serde_json::from_value(
            mapped_domain(
                &artifact.configuration,
                "sensor_calibration",
                &source_id,
                &target_id,
            )
            .unwrap(),
        )
        .unwrap();
        let mut expected_controller = serde_json::to_value(from_db_rows(
            &mapped_device,
            &mapped_water,
            &mapped_safety,
            &mapped_dosing,
            &mapped_sensor,
        ))
        .unwrap();
        let mut expected_sensor = serde_json::to_value(&mapped_sensor).unwrap();
        expected_controller["config_version"] = json!(version);
        expected_sensor["config_version"] = json!(version);

        assert_eq!(sync.config_version, version);
        assert_eq!(sync.controller_state, crate::db::config_sync::PENDING);
        assert_eq!(sync.sensor_state, crate::db::config_sync::PENDING);
        assert_eq!(sync.desired_controller_config, expected_controller);
        assert_eq!(sync.desired_sensor_config, expected_sensor);

        let mut rollback_tx = pool.begin().await.unwrap();
        let next_version = persist_restore_sync(
            &mut rollback_tx,
            &artifact.configuration,
            &source_id,
            &target_id,
        )
        .await
        .unwrap();
        assert_eq!(next_version, version + 1);
        rollback_tx.rollback().await.unwrap();

        let sync_after_rollback = crate::db::config_sync::get(&pool, &target_id)
            .await
            .unwrap()
            .expect("original restore sync row must remain");
        assert_eq!(sync_after_rollback.config_version, version);

        sqlx::query("DELETE FROM device_config WHERE device_id = $1")
            .bind(&target_id)
            .execute(&pool)
            .await
            .unwrap();
    }
}
