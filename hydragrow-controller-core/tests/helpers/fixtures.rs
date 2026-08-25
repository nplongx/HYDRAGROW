//! Shared test fixtures — ControllerConfig và SensorData cho tests

use hydragrow_controller_core::hydragrow_shared::{ControlMode, ControllerConfig, SensorData};

/// Config chuẩn: Auto mode, EC/pH sensor bật, targets trong ngưỡng
pub fn auto_config() -> ControllerConfig {
    ControllerConfig {
        device_id: "test_device".to_string(),
        control_mode: ControlMode::Auto,
        is_enabled: true,
        ec_target: 1.5,
        ec_tolerance: 0.05,
        ph_target: 6.0,
        ph_tolerance: 0.1,
        enable_ec_sensor: true,
        enable_ph_sensor: true,
        enable_water_level_sensor: true,
        enable_temp_sensor: false,
        water_level_min: 15.0,
        water_level_target: 20.0,
        water_level_max: 24.0,
        water_level_tolerance: 1.0,
        max_dose_per_hour: 50.0,
        max_dose_per_cycle: 10.0,
        cooldown_sec: 30,
        max_refill_cycles_per_hour: 4,
        max_drain_cycles_per_hour: 4,
        max_refill_duration_sec: 120,
        max_drain_duration_sec: 120,
        ec_gain_per_ml: 0.2,
        ph_shift_up_per_ml: 0.1,
        ph_shift_down_per_ml: 0.1,
        pump_a_capacity_ml_per_sec: 1.0,
        pump_b_capacity_ml_per_sec: 1.0,
        pump_ph_up_capacity_ml_per_sec: 0.5,
        pump_ph_down_capacity_ml_per_sec: 0.5,
        soft_start_duration: 100, // Ngắn để test nhanh
        delay_between_a_and_b_sec: 0,
        dosing_pwm_percent: 80,
        dosing_min_pwm_percent: 30,
        dosing_pulse_on_ms: 100,
        dosing_pulse_off_ms: 100,
        dosing_min_dose_ml: 0.1,
        dosing_max_pulse_count_per_cycle: 100,
        max_ec_delta: 0.5,
        max_ph_delta: 0.5,
        max_ec_limit: 3.0,
        min_ec_limit: 0.1,
        min_ph_limit: 4.5,
        max_ph_limit: 8.0,
        min_temp_limit: 15.0,
        max_temp_limit: 35.0,
        misting_temp_threshold: 30.0,
        ec_step_ratio: 1.0,
        ph_step_ratio: 1.0,
        best_ec_ratio: 1.0,
        best_ph_ratio: 1.0,
        adaptive_mixing_sec: 5,
        adaptive_stabilize_sec: 10,
        effective_ec_tolerance: 0.05,
        effective_ph_tolerance: 0.1,
        active_mixing_sec: 10,
        sensor_stabilize_sec: 10,
        scheduled_mixing_interval_sec: 3600,
        scheduled_mixing_duration_sec: 60,
        misting_on_duration_ms: 5000,
        misting_off_duration_ms: 30000,
        osaka_mixing_pwm_percent: 60,
        osaka_misting_pwm_percent: 100,
        high_temp_misting_on_duration_ms: 15000,
        high_temp_misting_off_duration_ms: 60000,
        ec_ack_threshold: 0.05,
        ph_ack_threshold: 0.1,
        water_ack_threshold: 0.5,
        water_level_critical_min: 10.0,
        auto_refill_enabled: true,
        auto_drain_overflow: true,
        auto_dilute_enabled: false,
        dilute_drain_amount_cm: 0.0,
        scheduled_water_change_enabled: false,
        water_change_cron: String::new(),
        scheduled_drain_amount_cm: 0.0,
        water_change_interval_days: None,
        emergency_shutdown: false,
        nutrient_a_ratio: 1.0,
        nutrient_b_ratio: 1.0,
        active_recipe: None,
        tuner_state: 0,
        interaction_matrix: None,
        matrix_update_count: 0,
        matrix_is_warm: false,
        kalman_confidence: None,
        tank_height: 30,
        pump_a_min_pwm_percent: None,
        pump_b_min_pwm_percent: None,
        pump_ph_up_min_pwm_percent: None,
        pump_ph_down_min_pwm_percent: None,
    }
}

/// Sensor data: EC và pH trong mức target — FSM sẽ Idle
pub fn balanced_sensors() -> SensorData {
    SensorData {
        device_id: "test_device".to_string(),
        ec: 1.5,
        ph: 6.0,
        temp: 25.0,
        water_level: 20.0,
        pump_status: Default::default(),
        time: "2026-08-25T10:00:00Z".to_string(),
        controller_received_ms: Some(1_000_000),
        rssi: Some(-60),
        free_heap: Some(100_000),
        uptime: Some(1000),
        err_water: None,
        err_temp: None,
        err_ec: None,
        err_ph: None,
        is_continuous: None,
        ph_voltage_mv: None,
    }
}

/// Sensor data: EC thấp hơn target → FSM sẽ trigger dosing
pub fn low_ec_sensors() -> SensorData {
    SensorData {
        ec: 1.0, // thấp hơn target 1.5 rõ ràng
        ph: 6.0,
        ..balanced_sensors()
    }
}

/// Sensor data: pH thấp hơn target → FSM sẽ trigger pH Up dosing
pub fn low_ph_sensors() -> SensorData {
    SensorData {
        ec: 1.5,
        ph: 5.5, // thấp hơn target 6.0, dưới tolerance 0.1
        ..balanced_sensors()
    }
}

/// Sensor data: Mực nước thấp → FSM sẽ trigger water refilling
pub fn low_water_sensors() -> SensorData {
    SensorData {
        ec: 1.5,
        ph: 6.0,
        water_level: 12.0, // thấp hơn water_level_min 15.0
        ..balanced_sensors()
    }
}

/// Sensor data giả lập noise (spike lớn bất thường)
pub fn noisy_ec_sensors(prev_ec: f32) -> SensorData {
    SensorData {
        ec: prev_ec + 1.0, // spike > max_ec_delta = 0.5
        ph: 6.0,
        ..balanced_sensors()
    }
}
