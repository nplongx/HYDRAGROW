-- Add migration script here
-- =============================================================================
-- Migration: Bổ sung ràng buộc nghiệp vụ cho các bảng đang sử dụng
-- Bảng loại bỏ: blockchain_history, blockchain_logs (không còn sử dụng)
-- =============================================================================

-- ---------------------------------------------------------------------------
-- 1. device_config
-- ---------------------------------------------------------------------------
ALTER TABLE device_config
    ADD CONSTRAINT chk_device_config_control_mode
        CHECK (control_mode IN ('auto', 'manual')),
    ADD CONSTRAINT chk_device_config_ec_target
        CHECK (ec_target > 0 AND ec_target <= 10.0),
    ADD CONSTRAINT chk_device_config_ec_tolerance
        CHECK (ec_tolerance >= 0 AND ec_tolerance < ec_target),
    ADD CONSTRAINT chk_device_config_ph_target
        CHECK (ph_target >= 0 AND ph_target <= 14.0),
    ADD CONSTRAINT chk_device_config_ph_tolerance
        CHECK (ph_tolerance >= 0 AND ph_tolerance < 7.0),
    ADD CONSTRAINT chk_device_config_delay_between_ab
        CHECK (delay_between_a_and_b_sec >= 0 AND delay_between_a_and_b_sec <= 600);

-- ---------------------------------------------------------------------------
-- 2. sensor_calibration
-- ---------------------------------------------------------------------------
ALTER TABLE sensor_calibration
    ADD CONSTRAINT chk_sensor_cal_ph_v7
        CHECK (ph_v7 > 0 AND ph_v7 < 5.0),
    ADD CONSTRAINT chk_sensor_cal_ph_v4
        CHECK (ph_v4 > 0 AND ph_v4 < 5.0 AND ph_v4 > ph_v7),
    ADD CONSTRAINT chk_sensor_cal_ph_v10
        CHECK (ph_v10 IS NULL OR (ph_v10 > 0 AND ph_v10 < 5.0 AND ph_v10 < ph_v7)),
    ADD CONSTRAINT chk_sensor_cal_calibration_mode
        CHECK (ph_calibration_mode IN ('2-point', '3-point')),
    ADD CONSTRAINT chk_sensor_cal_3point_requires_v10
        CHECK (ph_calibration_mode != '3-point' OR ph_v10 IS NOT NULL),
    ADD CONSTRAINT chk_sensor_cal_ec_factor
        CHECK (ec_factor > 0),
    ADD CONSTRAINT chk_sensor_cal_ec_offset
        CHECK (ec_offset BETWEEN -5.0 AND 5.0),
    ADD CONSTRAINT chk_sensor_cal_temp_offset
        CHECK (temp_offset BETWEEN -10.0 AND 10.0),
    ADD CONSTRAINT chk_sensor_cal_temp_beta
        CHECK (temp_compensation_beta BETWEEN 0.001 AND 0.1),
    ADD CONSTRAINT chk_sensor_cal_publish_interval
        CHECK (publish_interval BETWEEN 500 AND 300000),
    ADD CONSTRAINT chk_sensor_cal_moving_avg_window
        CHECK (moving_average_window BETWEEN 1 AND 200);

-- ---------------------------------------------------------------------------
-- 3. dosing_calibration
-- ---------------------------------------------------------------------------
ALTER TABLE dosing_calibration
    ADD CONSTRAINT chk_dosing_cal_ec_gain
        CHECK (ec_gain_per_ml > 0 AND ec_gain_per_ml <= 5.0),
    ADD CONSTRAINT chk_dosing_cal_ph_shift_up
        CHECK (ph_shift_up_per_ml > 0 AND ph_shift_up_per_ml <= 5.0),
    ADD CONSTRAINT chk_dosing_cal_ph_shift_down
        CHECK (ph_shift_down_per_ml > 0 AND ph_shift_down_per_ml <= 5.0),
    ADD CONSTRAINT chk_dosing_cal_active_mixing
        CHECK (active_mixing_sec >= 5 AND active_mixing_sec <= 3600),
    ADD CONSTRAINT chk_dosing_cal_stabilize
        CHECK (sensor_stabilize_sec >= 5 AND sensor_stabilize_sec <= 3600),
    ADD CONSTRAINT chk_dosing_cal_ec_step_ratio
        CHECK (ec_step_ratio > 0 AND ec_step_ratio <= 2.0),
    ADD CONSTRAINT chk_dosing_cal_ph_step_ratio
        CHECK (ph_step_ratio > 0 AND ph_step_ratio <= 2.0),
    ADD CONSTRAINT chk_dosing_cal_pump_a_capacity
        CHECK (pump_a_capacity_ml_per_sec > 0 AND pump_a_capacity_ml_per_sec <= 20.0),
    ADD CONSTRAINT chk_dosing_cal_pump_b_capacity
        CHECK (pump_b_capacity_ml_per_sec > 0 AND pump_b_capacity_ml_per_sec <= 20.0),
    ADD CONSTRAINT chk_dosing_cal_pump_ph_up_capacity
        CHECK (pump_ph_up_capacity_ml_per_sec > 0 AND pump_ph_up_capacity_ml_per_sec <= 20.0),
    ADD CONSTRAINT chk_dosing_cal_pump_ph_down_capacity
        CHECK (pump_ph_down_capacity_ml_per_sec > 0 AND pump_ph_down_capacity_ml_per_sec <= 20.0),
    ADD CONSTRAINT chk_dosing_cal_soft_start
        CHECK (soft_start_duration >= 0 AND soft_start_duration <= 10000),
    ADD CONSTRAINT chk_dosing_cal_mixing_interval
        CHECK (scheduled_mixing_interval_sec >= 0),
    ADD CONSTRAINT chk_dosing_cal_mixing_duration
        CHECK (scheduled_mixing_duration_sec >= 0
            AND (scheduled_mixing_interval_sec = 0
                 OR scheduled_mixing_duration_sec <= scheduled_mixing_interval_sec)),
    ADD CONSTRAINT chk_dosing_cal_dosing_pwm
        CHECK (dosing_pwm_percent BETWEEN 1 AND 100),
    ADD CONSTRAINT chk_dosing_cal_osaka_mixing_pwm
        CHECK (osaka_mixing_pwm_percent BETWEEN 0 AND 100),
    ADD CONSTRAINT chk_dosing_cal_osaka_misting_pwm
        CHECK (osaka_misting_pwm_percent BETWEEN 0 AND 100),
    ADD CONSTRAINT chk_dosing_cal_min_pwm
        CHECK (dosing_min_pwm_percent BETWEEN 0 AND 100),
    ADD CONSTRAINT chk_dosing_cal_min_pwm_lte_dosing_pwm
        CHECK (dosing_min_pwm_percent <= dosing_pwm_percent),
    ADD CONSTRAINT chk_dosing_cal_pulse_on_ms
        CHECK (dosing_pulse_on_ms >= 10 AND dosing_pulse_on_ms <= 60000),
    ADD CONSTRAINT chk_dosing_cal_pulse_off_ms
        CHECK (dosing_pulse_off_ms >= 0 AND dosing_pulse_off_ms <= 60000),
    ADD CONSTRAINT chk_dosing_cal_min_dose_ml
        CHECK (dosing_min_dose_ml > 0 AND dosing_min_dose_ml <= 10.0),
    ADD CONSTRAINT chk_dosing_cal_max_pulse_count
        CHECK (dosing_max_pulse_count_per_cycle >= 1 AND dosing_max_pulse_count_per_cycle <= 500);

-- ---------------------------------------------------------------------------
-- 4. water_config
-- ---------------------------------------------------------------------------
ALTER TABLE water_config
    ADD CONSTRAINT chk_water_config_tank_height
        CHECK (tank_height BETWEEN 5 AND 500),
    ADD CONSTRAINT chk_water_config_level_order
        CHECK (water_level_min < water_level_target
           AND water_level_target < water_level_max),
    ADD CONSTRAINT chk_water_config_drain_level
        CHECK (water_level_drain >= 0 AND water_level_drain <= water_level_min),
    ADD CONSTRAINT chk_water_config_level_tolerance
        CHECK (water_level_tolerance > 0
           AND water_level_tolerance < (water_level_target - water_level_min)),
    ADD CONSTRAINT chk_water_config_dilute_drain_amount
        CHECK (dilute_drain_amount_cm > 0 AND dilute_drain_amount_cm <= water_level_max),
    ADD CONSTRAINT chk_water_config_scheduled_drain_amount
        CHECK (scheduled_drain_amount_cm > 0 AND scheduled_drain_amount_cm <= water_level_max),
    ADD CONSTRAINT chk_water_config_misting_on
        CHECK (misting_on_duration_ms >= 1000),
    ADD CONSTRAINT chk_water_config_misting_off
        CHECK (misting_off_duration_ms >= 1000),
    ADD CONSTRAINT chk_water_config_high_temp_misting_on
        CHECK (high_temp_misting_on_duration_ms >= 1000),
    ADD CONSTRAINT chk_water_config_high_temp_misting_off
        CHECK (high_temp_misting_off_duration_ms >= 1000),
    ADD CONSTRAINT chk_water_config_misting_temp_threshold
        CHECK (misting_temp_threshold BETWEEN 20.0 AND 60.0),
    ADD CONSTRAINT chk_water_config_cron_when_enabled
        CHECK (NOT scheduled_water_change_enabled OR length(trim(water_change_cron)) > 0);

-- ---------------------------------------------------------------------------
-- 5. safety_config
-- ---------------------------------------------------------------------------
ALTER TABLE safety_config
    ADD CONSTRAINT chk_safety_ec_limits
        CHECK (min_ec_limit >= 0 AND max_ec_limit > min_ec_limit),
    ADD CONSTRAINT chk_safety_ph_limits
        CHECK (min_ph_limit >= 0 AND max_ph_limit <= 14.0
           AND min_ph_limit < max_ph_limit),
    ADD CONSTRAINT chk_safety_ec_delta
        CHECK (max_ec_delta > 0 AND max_ec_delta <= 5.0),
    ADD CONSTRAINT chk_safety_ph_delta
        CHECK (max_ph_delta > 0 AND max_ph_delta <= 7.0),
    ADD CONSTRAINT chk_safety_max_dose_per_cycle
        CHECK (max_dose_per_cycle BETWEEN 0.1 AND 500.0),
    ADD CONSTRAINT chk_safety_max_dose_per_hour
        CHECK (max_dose_per_hour >= max_dose_per_cycle),
    ADD CONSTRAINT chk_safety_cooldown
        CHECK (cooldown_sec BETWEEN 0 AND 3600),
    ADD CONSTRAINT chk_safety_temp_limits
        CHECK (min_temp_limit >= -10.0 AND max_temp_limit <= 80.0
           AND min_temp_limit < max_temp_limit),
    ADD CONSTRAINT chk_safety_water_critical_min
        CHECK (water_level_critical_min > 0),
    ADD CONSTRAINT chk_safety_refill_cycles
        CHECK (max_refill_cycles_per_hour BETWEEN 1 AND 100),
    ADD CONSTRAINT chk_safety_drain_cycles
        CHECK (max_drain_cycles_per_hour BETWEEN 1 AND 100),
    ADD CONSTRAINT chk_safety_refill_duration
        CHECK (max_refill_duration_sec BETWEEN 10 AND 7200),
    ADD CONSTRAINT chk_safety_drain_duration
        CHECK (max_drain_duration_sec BETWEEN 10 AND 7200),
    ADD CONSTRAINT chk_safety_ec_ack_threshold
        CHECK (ec_ack_threshold > 0 AND ec_ack_threshold <= 2.0),
    ADD CONSTRAINT chk_safety_ph_ack_threshold
        CHECK (ph_ack_threshold > 0 AND ph_ack_threshold <= 5.0),
    ADD CONSTRAINT chk_safety_water_ack_threshold
        CHECK (water_ack_threshold > 0 AND water_ack_threshold <= 20.0);

-- ---------------------------------------------------------------------------
-- 6. crop_seasons
-- ---------------------------------------------------------------------------
ALTER TABLE crop_seasons
    ADD CONSTRAINT chk_crop_season_status
        CHECK (status IN ('active', 'completed')),
    ADD CONSTRAINT chk_crop_season_name_not_empty
        CHECK (length(trim(name)) > 0),
    ADD CONSTRAINT chk_crop_season_time_order
        CHECK (end_time IS NULL OR end_time > start_time),
    ADD CONSTRAINT chk_crop_season_completed_has_end_time
        CHECK (status != 'completed' OR end_time IS NOT NULL);

CREATE UNIQUE INDEX IF NOT EXISTS uq_crop_seasons_one_active_per_device
    ON crop_seasons (device_id)
    WHERE status = 'active';

-- ---------------------------------------------------------------------------
-- 7. dosing_reports
-- ---------------------------------------------------------------------------
ALTER TABLE dosing_reports
    ADD CONSTRAINT chk_dosing_report_pump_a_ml
        CHECK (pump_a_ml >= 0),
    ADD CONSTRAINT chk_dosing_report_pump_b_ml
        CHECK (pump_b_ml >= 0),
    ADD CONSTRAINT chk_dosing_report_ph_up_ml
        CHECK (ph_up_ml >= 0),
    ADD CONSTRAINT chk_dosing_report_ph_down_ml
        CHECK (ph_down_ml >= 0),
    ADD CONSTRAINT chk_dosing_report_at_least_one_pump
        CHECK (pump_a_ml > 0 OR pump_b_ml > 0 OR ph_up_ml > 0 OR ph_down_ml > 0),
    ADD CONSTRAINT chk_dosing_report_no_ph_conflict
        CHECK (NOT (ph_up_ml > 0 AND ph_down_ml > 0)),
    ADD CONSTRAINT chk_dosing_report_payload_not_empty
        CHECK (payload != 'null'::jsonb AND payload != '{}'::jsonb);

-- ---------------------------------------------------------------------------
-- 8. system_events
-- ---------------------------------------------------------------------------
ALTER TABLE system_events
    ADD CONSTRAINT chk_system_event_level
        CHECK (level IN ('info', 'success', 'warning', 'critical', 'error')),
    ADD CONSTRAINT chk_system_event_category
        CHECK (category IN ('system', 'dosing', 'water', 'alert', 'calibration', 'sensor')),
    ADD CONSTRAINT chk_system_event_title_not_empty
        CHECK (length(trim(title)) > 0),
    ADD CONSTRAINT chk_system_event_message_not_empty
        CHECK (length(trim(message)) > 0),
    ADD CONSTRAINT chk_system_event_timestamp_valid
        CHECK (timestamp > 1577836800000);
