use chrono::{DateTime, Utc};
use hydragrow_shared::{ControlMode, ControllerConfig};
use serde::{Deserialize, Serialize};
use sqlx::prelude::FromRow;

#[derive(Debug, Serialize, Deserialize, FromRow)]
pub struct DeviceConfig {
    pub device_id: String,
    pub ec_target: f32,
    pub ec_tolerance: f32,
    pub ph_tolerance: f32,
    pub ph_target: f32,
    pub control_mode: String,
    pub is_enabled: bool,
    pub delay_between_a_and_b_sec: i32,
    pub last_updated: DateTime<Utc>,
}

#[derive(Debug, Serialize, Deserialize, FromRow)]
pub struct SensorCalibration {
    pub device_id: String,
    pub ph_v7: f32,
    pub ph_v4: f32,
    pub ph_v10: Option<f32>,         // 🟢 THÊM MỚI
    pub ph_calibration_mode: String, // 🟢 THÊM MỚI: "2-point" hoặc "3-point"
    pub ec_factor: f32,
    pub ec_offset: f32,
    pub temp_offset: f32,
    pub temp_compensation_beta: f32,
    pub publish_interval: i32,
    pub moving_average_window: i32,
    pub enable_ph_sensor: bool,
    pub enable_ec_sensor: bool,
    pub enable_temp_sensor: bool,
    pub enable_water_level_sensor: bool,
    pub last_calibrated: DateTime<Utc>,
}

#[derive(Debug, Serialize, Deserialize, FromRow)]
pub struct PumpCalibration {
    pub id: String,
    pub device_id: String,
    pub pump_type: String,
    pub flow_rate_ml_per_sec: f32,
    pub min_activation_sec: f32,
    pub max_activation_sec: f32,
    pub last_calibrated: DateTime<Utc>,
}

#[derive(Debug, Serialize, Deserialize, FromRow)]
pub struct DosingCalibration {
    pub device_id: String,
    pub ec_gain_per_ml: f32,
    pub ph_shift_up_per_ml: f32,
    pub ph_shift_down_per_ml: f32,
    pub active_mixing_sec: i32,
    pub sensor_stabilize_sec: i32,
    pub ec_step_ratio: f32,
    pub ph_step_ratio: f32,
    #[serde(default = "default_best_ec_ratio")]
    pub best_ec_ratio: f32,
    #[serde(default = "default_best_ph_ratio")]
    pub best_ph_ratio: f32,
    #[serde(default)]
    pub tuner_state: i32,
    #[serde(default)]
    pub interaction_matrix: Option<serde_json::Value>,
    #[serde(default)]
    pub matrix_update_count: i32,
    #[serde(default)]
    pub matrix_is_warm: bool,
    #[serde(default)]
    pub kalman_confidence: Option<serde_json::Value>,

    pub pump_a_capacity_ml_per_sec: f32,
    pub pump_b_capacity_ml_per_sec: f32,
    pub pump_ph_up_capacity_ml_per_sec: f32,
    pub pump_ph_down_capacity_ml_per_sec: f32,

    pub dosing_min_pwm_percent: i32,
    pub pump_a_min_pwm_percent: Option<i32>,
    pub pump_b_min_pwm_percent: Option<i32>,
    pub pump_ph_up_min_pwm_percent: Option<i32>,
    pub pump_ph_down_min_pwm_percent: Option<i32>,
    pub dosing_pulse_on_ms: i32,
    pub dosing_pulse_off_ms: i32,
    pub dosing_min_dose_ml: f32,
    pub dosing_max_pulse_count_per_cycle: i32,

    pub soft_start_duration: i32,
    pub last_calibrated: DateTime<Utc>,
    pub scheduled_mixing_interval_sec: i32,
    pub scheduled_mixing_duration_sec: i32,

    pub dosing_pwm_percent: i32,
    pub osaka_mixing_pwm_percent: i32,
    pub osaka_misting_pwm_percent: i32,
    // pub ec_gain_dynamic: f32,
    // pub ph_up_dynamic: f32,
    // pub ph_down_dynamic: f32,
    // pub dynamic_sample_count: i32,
    // pub dynamic_confidence: f32,
    // pub last_dynamic_update: Option<DateTime<Utc>>,
    // pub dynamic_model_version: String,
}

impl Default for DosingCalibration {
    fn default() -> Self {
        Self {
            device_id: String::new(),
            ec_gain_per_ml: 0.01,
            ph_shift_up_per_ml: 0.01,
            ph_shift_down_per_ml: 0.01,
            active_mixing_sec: 30,
            sensor_stabilize_sec: 10,
            ec_step_ratio: 0.1,
            ph_step_ratio: 0.1,
            best_ec_ratio: 0.4,
            best_ph_ratio: 0.2,
            tuner_state: 0,
            interaction_matrix: None,
            matrix_update_count: 0,
            matrix_is_warm: false,
            kalman_confidence: None,

            pump_a_capacity_ml_per_sec: 1.2,
            pump_b_capacity_ml_per_sec: 1.2,
            pump_ph_up_capacity_ml_per_sec: 1.2,
            pump_ph_down_capacity_ml_per_sec: 1.2,

            dosing_min_pwm_percent: 20,
            pump_a_min_pwm_percent: Some(20),
            pump_b_min_pwm_percent: Some(20),
            pump_ph_up_min_pwm_percent: Some(20),
            pump_ph_down_min_pwm_percent: Some(20),

            dosing_pulse_on_ms: 500,
            dosing_pulse_off_ms: 500,
            dosing_min_dose_ml: 1.0,
            dosing_max_pulse_count_per_cycle: 20,

            soft_start_duration: 5,
            last_calibrated: Utc::now(),
            scheduled_mixing_interval_sec: 600,
            scheduled_mixing_duration_sec: 60,

            dosing_pwm_percent: 50,
            osaka_mixing_pwm_percent: 60,
            osaka_misting_pwm_percent: 100,
            // ec_gain_dynamic: 0.01,
            // ph_up_dynamic: 0.01,
            // ph_down_dynamic: 0.01,
            // dynamic_sample_count: 0,
            // dynamic_confidence: 0.0,
            // last_dynamic_update: None,
            // dynamic_model_version: "v1".to_string(),
        }
    }
}

fn default_best_ec_ratio() -> f32 {
    0.4
}

fn default_best_ph_ratio() -> f32 {
    0.2
}

#[derive(Debug, Serialize, Deserialize, FromRow)]
pub struct SafetyConfig {
    pub device_id: String,
    pub max_ec_limit: f32,
    pub min_ec_limit: f32,
    pub min_ph_limit: f32,
    pub max_ph_limit: f32,
    pub max_ec_delta: f32,
    pub max_ph_delta: f32,
    pub max_dose_per_cycle: f32,
    pub cooldown_sec: i32,
    pub max_dose_per_hour: f32,
    pub water_level_critical_min: f32,
    pub max_refill_cycles_per_hour: i32,
    pub max_drain_cycles_per_hour: i32,
    pub max_refill_duration_sec: i32,
    pub max_drain_duration_sec: i32,
    pub min_temp_limit: f32,
    pub max_temp_limit: f32,
    pub emergency_shutdown: bool,
    pub ec_ack_threshold: f32,
    pub ph_ack_threshold: f32,
    pub water_ack_threshold: f32,

    pub last_updated: DateTime<Utc>,
}

impl Default for SafetyConfig {
    fn default() -> Self {
        Self {
            device_id: String::new(),
            max_ec_limit: 3.0,
            min_ec_limit: 0.5,
            min_ph_limit: 5.5,
            max_ph_limit: 6.5,
            max_ec_delta: 0.5,
            max_ph_delta: 0.5,
            max_dose_per_cycle: 50.0,
            cooldown_sec: 60,
            max_dose_per_hour: 200.0,
            water_level_critical_min: 5.0,
            max_refill_cycles_per_hour: 10,
            max_drain_cycles_per_hour: 10,
            max_refill_duration_sec: 120,
            max_drain_duration_sec: 120,
            min_temp_limit: 15.0,
            max_temp_limit: 35.0,
            emergency_shutdown: false,
            ec_ack_threshold: 0.2,
            ph_ack_threshold: 0.2,
            water_ack_threshold: 1.0,
            last_updated: Utc::now(),
        }
    }
}

#[derive(Debug, Serialize, Deserialize, Clone, FromRow)]
pub struct WaterConfig {
    pub device_id: String,
    pub tank_height: i32,
    pub water_level_min: f32,
    pub water_level_target: f32,
    pub water_level_max: f32,
    pub water_level_drain: f32,
    // pub circulation_mode: String,
    // pub circulation_on_sec: i32,
    // pub circulation_off_sec: i32,
    pub water_level_tolerance: f32,
    pub auto_refill_enabled: bool,
    pub auto_drain_overflow: bool,
    pub auto_dilute_enabled: bool,
    pub dilute_drain_amount_cm: f32,
    pub scheduled_water_change_enabled: bool,
    pub water_change_cron: String,
    pub scheduled_drain_amount_cm: f32,
    pub misting_on_duration_ms: i32,
    pub misting_off_duration_ms: i32,

    pub misting_temp_threshold: f32,
    pub high_temp_misting_on_duration_ms: i64,
    pub high_temp_misting_off_duration_ms: i64,

    pub last_updated: DateTime<Utc>,
}

impl Default for WaterConfig {
    fn default() -> Self {
        Self {
            device_id: String::new(),
            tank_height: 50,
            water_level_min: 10.0,
            water_level_target: 20.0,
            water_level_max: 30.0,
            water_level_drain: 5.0,
            // circulation_mode: "auto".to_string(),
            // circulation_on_sec: 60,
            // circulation_off_sec: 300,
            water_level_tolerance: 1.0,
            auto_refill_enabled: true,
            auto_drain_overflow: true,
            auto_dilute_enabled: false,
            dilute_drain_amount_cm: 2.0,
            scheduled_water_change_enabled: false,
            water_change_cron: "0 0 7 * * SUN".to_string(),
            scheduled_drain_amount_cm: 5.0,
            misting_on_duration_ms: 5000,
            misting_off_duration_ms: 10000,

            misting_temp_threshold: 30.0,
            high_temp_misting_off_duration_ms: 10000,
            high_temp_misting_on_duration_ms: 180000,

            last_updated: Utc::now(),
        }
    }
}

pub fn from_db_rows(
    dev: &DeviceConfig,
    water: &WaterConfig,
    safe: &SafetyConfig,
    dose: &DosingCalibration,
    sens: &SensorCalibration,
) -> ControllerConfig {
    ControllerConfig {
        device_id: dev.device_id.clone(),
        control_mode: ControlMode::from_string(&dev.control_mode),
        is_enabled: dev.is_enabled,
        delay_between_a_and_b_sec: dev.delay_between_a_and_b_sec,
        ec_target: dev.ec_target,
        ec_tolerance: dev.ec_tolerance,
        ph_target: dev.ph_target,
        ph_tolerance: dev.ph_tolerance,
        water_level_min: water.water_level_min,
        water_level_target: water.water_level_target,
        water_level_max: water.water_level_max,
        water_level_tolerance: water.water_level_tolerance,
        auto_refill_enabled: water.auto_refill_enabled,
        auto_drain_overflow: water.auto_drain_overflow,
        auto_dilute_enabled: water.auto_dilute_enabled,
        dilute_drain_amount_cm: water.dilute_drain_amount_cm,
        scheduled_water_change_enabled: water.scheduled_water_change_enabled,
        water_change_cron: water.water_change_cron.clone(),
        scheduled_drain_amount_cm: water.scheduled_drain_amount_cm,
        misting_on_duration_ms: water.misting_on_duration_ms,
        misting_off_duration_ms: water.misting_off_duration_ms,
        emergency_shutdown: safe.emergency_shutdown,
        max_ec_limit: safe.max_ec_limit,
        min_ec_limit: safe.min_ec_limit,
        min_ph_limit: safe.min_ph_limit,
        max_ph_limit: safe.max_ph_limit,
        max_ec_delta: safe.max_ec_delta,
        max_ph_delta: safe.max_ph_delta,
        min_temp_limit: safe.min_temp_limit,
        max_temp_limit: safe.max_temp_limit,

        max_dose_per_cycle: safe.max_dose_per_cycle,
        max_dose_per_hour: safe.max_dose_per_hour,
        cooldown_sec: safe.cooldown_sec,
        max_refill_cycles_per_hour: safe.max_refill_cycles_per_hour,
        max_drain_cycles_per_hour: safe.max_drain_cycles_per_hour,

        water_level_critical_min: safe.water_level_critical_min,

        max_refill_duration_sec: safe.max_refill_duration_sec,
        max_drain_duration_sec: safe.max_drain_duration_sec,

        ec_ack_threshold: safe.ec_ack_threshold,
        ph_ack_threshold: safe.ph_ack_threshold,
        water_ack_threshold: safe.water_ack_threshold,
        ec_gain_per_ml: dose.ec_gain_per_ml,
        ph_shift_up_per_ml: dose.ph_shift_up_per_ml,
        ph_shift_down_per_ml: dose.ph_shift_down_per_ml,
        active_mixing_sec: dose.active_mixing_sec,
        sensor_stabilize_sec: dose.sensor_stabilize_sec,
        ec_step_ratio: dose.ec_step_ratio,
        ph_step_ratio: dose.ph_step_ratio,
        best_ec_ratio: dose.best_ec_ratio,
        best_ph_ratio: dose.best_ph_ratio,
        tuner_state: dose.tuner_state.clamp(0, u8::MAX as i32) as u8,
        interaction_matrix: dose
            .interaction_matrix
            .as_ref()
            .and_then(json_array_to_f32_vec),
        matrix_update_count: dose.matrix_update_count.max(0) as u32,
        matrix_is_warm: dose.matrix_is_warm,
        kalman_confidence: dose
            .kalman_confidence
            .as_ref()
            .and_then(json_array_to_f32_vec),

        pump_a_capacity_ml_per_sec: dose.pump_a_capacity_ml_per_sec,
        pump_b_capacity_ml_per_sec: dose.pump_b_capacity_ml_per_sec,
        pump_ph_up_capacity_ml_per_sec: dose.pump_ph_up_capacity_ml_per_sec,
        pump_ph_down_capacity_ml_per_sec: dose.pump_ph_down_capacity_ml_per_sec,

        soft_start_duration: dose.soft_start_duration,
        scheduled_mixing_interval_sec: dose.scheduled_mixing_interval_sec,
        scheduled_mixing_duration_sec: dose.scheduled_mixing_duration_sec,

        enable_ec_sensor: sens.enable_ec_sensor,
        enable_ph_sensor: sens.enable_ph_sensor,
        enable_water_level_sensor: sens.enable_water_level_sensor,
        enable_temp_sensor: sens.enable_temp_sensor,

        tank_height: water.tank_height,

        dosing_pwm_percent: dose.dosing_pwm_percent,
        osaka_mixing_pwm_percent: dose.osaka_mixing_pwm_percent,
        osaka_misting_pwm_percent: dose.osaka_misting_pwm_percent,

        dosing_min_pwm_percent: dose.dosing_min_pwm_percent,
        pump_a_min_pwm_percent: dose.pump_a_min_pwm_percent,
        pump_b_min_pwm_percent: dose.pump_b_min_pwm_percent,
        pump_ph_up_min_pwm_percent: dose.pump_ph_up_min_pwm_percent,
        pump_ph_down_min_pwm_percent: dose.pump_ph_down_min_pwm_percent,
        dosing_pulse_on_ms: dose.dosing_pulse_on_ms,
        dosing_pulse_off_ms: dose.dosing_pulse_off_ms,
        dosing_min_dose_ml: dose.dosing_min_dose_ml,
        dosing_max_pulse_count_per_cycle: dose.dosing_max_pulse_count_per_cycle,
        misting_temp_threshold: water.misting_temp_threshold,
        high_temp_misting_on_duration_ms: water.high_temp_misting_on_duration_ms,
        high_temp_misting_off_duration_ms: water.high_temp_misting_off_duration_ms,
    }
}

fn json_array_to_f32_vec(value: &serde_json::Value) -> Option<Vec<f32>> {
    let items = value.as_array()?;
    let mut out = Vec::with_capacity(items.len());
    for item in items {
        let number = item.as_f64()?;
        if !number.is_finite() {
            return None;
        }
        out.push(number as f32);
    }
    Some(out)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn base_rows() -> (
        DeviceConfig,
        WaterConfig,
        SafetyConfig,
        DosingCalibration,
        SensorCalibration,
    ) {
        (
            DeviceConfig {
                device_id: "device-1".to_string(),
                ec_target: 1.6,
                ec_tolerance: 0.1,
                ph_tolerance: 0.2,
                ph_target: 6.0,
                control_mode: "auto".to_string(),
                is_enabled: true,
                delay_between_a_and_b_sec: 10,
                last_updated: Utc::now(),
            },
            WaterConfig {
                device_id: "device-1".to_string(),
                ..Default::default()
            },
            SafetyConfig {
                device_id: "device-1".to_string(),
                ..Default::default()
            },
            DosingCalibration {
                device_id: "device-1".to_string(),
                best_ec_ratio: 0.77,
                best_ph_ratio: 0.33,
                tuner_state: 2,
                interaction_matrix: Some(serde_json::json!(
                    (0..32).map(|i| i as f32 * 0.01).collect::<Vec<_>>()
                )),
                matrix_update_count: 12,
                matrix_is_warm: true,
                kalman_confidence: Some(serde_json::json!([
                    0.1, 0.2, 0.3, 0.4, 0.5, 0.6, 0.7, 0.8
                ])),
                ..Default::default()
            },
            SensorCalibration {
                device_id: "device-1".to_string(),
                ph_v7: 2.5,
                ph_v4: 1.4,
                ph_v10: None,
                ph_calibration_mode: "2-point".to_string(),
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
            },
        )
    }

    #[test]
    fn from_db_rows_preserves_runtime_learning_fields_for_config_sync() {
        let (dev, water, safe, dose, sens) = base_rows();

        let config = from_db_rows(&dev, &water, &safe, &dose, &sens);

        assert_eq!(config.best_ec_ratio, 0.77);
        assert_eq!(config.best_ph_ratio, 0.33);
        assert_eq!(config.tuner_state, 2);
        assert_eq!(config.matrix_update_count, 12);
        assert!(config.matrix_is_warm);
        assert_eq!(config.interaction_matrix.as_ref().unwrap().len(), 32);
        assert_eq!(config.kalman_confidence.as_ref().unwrap().len(), 8);
    }

    #[test]
    fn dosing_calibration_deserializes_legacy_payload_without_runtime_learning_fields() {
        let json = serde_json::json!({
            "device_id": "device-1",
            "ec_gain_per_ml": 0.01,
            "ph_shift_up_per_ml": 0.01,
            "ph_shift_down_per_ml": 0.01,
            "active_mixing_sec": 30,
            "sensor_stabilize_sec": 10,
            "ec_step_ratio": 0.1,
            "ph_step_ratio": 0.1,
            "pump_a_capacity_ml_per_sec": 1.2,
            "pump_b_capacity_ml_per_sec": 1.2,
            "pump_ph_up_capacity_ml_per_sec": 1.2,
            "pump_ph_down_capacity_ml_per_sec": 1.2,
            "dosing_min_pwm_percent": 20,
            "dosing_pulse_on_ms": 500,
            "dosing_pulse_off_ms": 500,
            "dosing_min_dose_ml": 1.0,
            "dosing_max_pulse_count_per_cycle": 20,
            "soft_start_duration": 5,
            "last_calibrated": Utc::now(),
            "scheduled_mixing_interval_sec": 600,
            "scheduled_mixing_duration_sec": 60,
            "dosing_pwm_percent": 50,
            "osaka_mixing_pwm_percent": 60,
            "osaka_misting_pwm_percent": 100
        });

        let decoded: DosingCalibration = serde_json::from_value(json).expect("legacy payload");

        assert_eq!(decoded.best_ec_ratio, 0.4);
        assert_eq!(decoded.best_ph_ratio, 0.2);
        assert_eq!(decoded.tuner_state, 0);
        assert!(decoded.interaction_matrix.is_none());
        assert_eq!(decoded.matrix_update_count, 0);
        assert!(!decoded.matrix_is_warm);
        assert!(decoded.kalman_confidence.is_none());
    }
}
