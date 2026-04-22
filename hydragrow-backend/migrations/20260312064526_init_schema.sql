-- Đã xóa PRAGMA foreign_keys = ON; vì Postgres mặc định quản lý Khóa ngoại tự động

-- 1. DEVICE CONFIG
CREATE TABLE device_config (
    device_id TEXT NOT NULL PRIMARY KEY,
    ec_target REAL NOT NULL,
    ec_tolerance REAL NOT NULL,
    ph_target REAL NOT NULL,
    ph_tolerance REAL NOT NULL,
    temp_target REAL NOT NULL,
    temp_tolerance REAL NOT NULL,
    control_mode TEXT NOT NULL,
    is_enabled BOOLEAN NOT NULL DEFAULT TRUE, -- Chuyển từ INTEGER sang BOOLEAN
    
    pump_a_capacity_ml_per_sec REAL NOT NULL DEFAULT 1.2,
    pump_b_capacity_ml_per_sec REAL NOT NULL DEFAULT 1.2,
    delay_between_a_and_b_sec INTEGER NOT NULL DEFAULT 10,

    last_updated TIMESTAMPTZ NOT NULL DEFAULT CURRENT_TIMESTAMP -- Chuyển TEXT sang TIMESTAMPTZ
);

-- 2. SENSOR CALIBRATION
CREATE TABLE sensor_calibration (
    device_id TEXT NOT NULL PRIMARY KEY,
    ph_v7 REAL NOT NULL,
    ph_v4 REAL NOT NULL,
    ec_factor REAL NOT NULL,
    ec_offset REAL NOT NULL DEFAULT 0.0,
    temp_offset REAL NOT NULL,
    temp_compensation_beta REAL NOT NULL DEFAULT 0.02,
    sampling_interval INTEGER NOT NULL DEFAULT 1000,
    publish_interval INTEGER NOT NULL DEFAULT 5000,
    moving_average_window INTEGER NOT NULL DEFAULT 10,
    is_ph_enabled BOOLEAN NOT NULL DEFAULT TRUE,
    is_ec_enabled BOOLEAN NOT NULL DEFAULT TRUE,
    is_temp_enabled BOOLEAN NOT NULL DEFAULT TRUE,
    is_water_level_enabled BOOLEAN NOT NULL DEFAULT TRUE,
    last_calibrated TIMESTAMPTZ NOT NULL DEFAULT CURRENT_TIMESTAMP,
    FOREIGN KEY (device_id) REFERENCES device_config(device_id) ON DELETE CASCADE
);

-- 3. PUMP CALIBRATION
CREATE TABLE pump_calibration (
    id TEXT NOT NULL PRIMARY KEY,
    device_id TEXT NOT NULL,
    pump_type TEXT NOT NULL CHECK(pump_type IN ('A','B','PH_UP','PH_DOWN')),
    flow_rate_ml_per_sec REAL NOT NULL,
    min_activation_sec REAL NOT NULL,
    max_activation_sec REAL NOT NULL,
    last_calibrated TIMESTAMPTZ NOT NULL DEFAULT CURRENT_TIMESTAMP,
    FOREIGN KEY (device_id) REFERENCES device_config(device_id) ON DELETE CASCADE
);

-- 4. DOSING CALIBRATION
CREATE TABLE dosing_calibration (
    device_id TEXT NOT NULL PRIMARY KEY,
    tank_volume_l REAL NOT NULL,
    ec_gain_per_ml REAL NOT NULL,
    ph_shift_up_per_ml REAL NOT NULL,
    ph_shift_down_per_ml REAL NOT NULL,
    active_mixing_sec INTEGER NOT NULL,
    sensor_stabilize_sec INTEGER NOT NULL,
    ec_step_ratio REAL NOT NULL,
    ph_step_ratio REAL NOT NULL,
    dosing_pump_capacity_ml_per_sec REAL NOT NULL DEFAULT 0.5,
    soft_start_duration INTEGER NOT NULL DEFAULT 3000,
    scheduled_mixing_interval_sec INTEGER NOT NULL DEFAULT 3600,
    scheduled_mixing_duration_sec INTEGER NOT NULL DEFAULT 300,

    dosing_pwm_percent INTEGER NOT NULL DEFAULT 50,
    osaka_mixing_pwm_percent INTEGER NOT NULL DEFAULT 60,
    osaka_misting_pwm_percent INTEGER NOT NULL DEFAULT 100,

    last_calibrated TIMESTAMPTZ NOT NULL DEFAULT CURRENT_TIMESTAMP,
    FOREIGN KEY (device_id) REFERENCES device_config(device_id) ON DELETE CASCADE
);

-- 5. WATER CONFIG
CREATE TABLE water_config (
    device_id TEXT NOT NULL PRIMARY KEY,
    water_level_min REAL NOT NULL,
    water_level_target REAL NOT NULL,
    water_level_max REAL NOT NULL,
    water_level_drain REAL NOT NULL,

    circulation_mode TEXT NOT NULL,
    circulation_on_sec INTEGER NOT NULL,
    circulation_off_sec INTEGER NOT NULL,

    water_level_tolerance REAL NOT NULL DEFAULT 1.0,
    auto_refill_enabled BOOLEAN NOT NULL DEFAULT TRUE,
    auto_drain_overflow BOOLEAN NOT NULL DEFAULT TRUE,
    auto_dilute_enabled BOOLEAN NOT NULL DEFAULT TRUE,
    dilute_drain_amount_cm REAL NOT NULL DEFAULT 2.0,
    scheduled_water_change_enabled BOOLEAN NOT NULL DEFAULT FALSE,
    water_change_interval_sec INTEGER NOT NULL DEFAULT 259200,
    scheduled_drain_amount_cm REAL NOT NULL DEFAULT 5.0,
    misting_on_duration_ms INTEGER NOT NULL DEFAULT 10000,
    misting_off_duration_ms INTEGER NOT NULL DEFAULT 180000,
    last_updated TIMESTAMPTZ NOT NULL DEFAULT CURRENT_TIMESTAMP,
    FOREIGN KEY (device_id) REFERENCES device_config(device_id) ON DELETE CASCADE
);

-- 6. SAFETY CONFIG
CREATE TABLE safety_config (
    device_id TEXT NOT NULL PRIMARY KEY,
    min_ec_limit REAL NOT NULL,
    max_ec_limit REAL NOT NULL,
    min_ph_limit REAL NOT NULL,
    max_ph_limit REAL NOT NULL,
    max_ec_delta REAL NOT NULL,
    max_ph_delta REAL NOT NULL,
    max_dose_per_cycle REAL NOT NULL,
    max_dose_per_hour REAL NOT NULL,
    cooldown_sec INTEGER NOT NULL,
    min_temp_limit REAL NOT NULL,
    max_temp_limit REAL NOT NULL,
    water_level_critical_min REAL NOT NULL,
    max_refill_cycles_per_hour INTEGER NOT NULL,
    max_drain_cycles_per_hour INTEGER NOT NULL,
    max_refill_duration_sec INTEGER NOT NULL,
    max_drain_duration_sec INTEGER NOT NULL,
    emergency_shutdown BOOLEAN NOT NULL DEFAULT FALSE,
    ec_ack_threshold REAL NOT NULL DEFAULT 0.05,
    ph_ack_threshold REAL NOT NULL DEFAULT 0.1,
    water_ack_threshold REAL NOT NULL DEFAULT 0.5,
    last_updated TIMESTAMPTZ NOT NULL DEFAULT CURRENT_TIMESTAMP,
    FOREIGN KEY (device_id) REFERENCES device_config(device_id) ON DELETE CASCADE
);

-- 7. BLOCKCHAIN HISTORY
CREATE TABLE blockchain_history (
    id SERIAL PRIMARY KEY, -- Chuyển AUTOINCREMENT sang SERIAL
    device_id TEXT NOT NULL,
    action TEXT NOT NULL,
    tx_id TEXT NOT NULL UNIQUE,
    explorer_url TEXT NOT NULL,
    created_at TIMESTAMPTZ NOT NULL DEFAULT CURRENT_TIMESTAMP,
    FOREIGN KEY (device_id) REFERENCES device_config(device_id) ON DELETE CASCADE
);

-- 8. CROP SEASONS
CREATE TABLE crop_seasons (
    id TEXT PRIMARY KEY NOT NULL,
    device_id TEXT NOT NULL,
    name TEXT NOT NULL,
    plant_type TEXT,
    start_time TIMESTAMPTZ NOT NULL DEFAULT CURRENT_TIMESTAMP,
    end_time TIMESTAMPTZ, 
    status TEXT NOT NULL DEFAULT 'active',
    FOREIGN KEY (device_id) REFERENCES device_config(device_id)
);

-- 9. BLOCKCHAIN LOGS
CREATE TABLE IF NOT EXISTS blockchain_logs (
    id SERIAL PRIMARY KEY, -- Chuyển AUTOINCREMENT sang SERIAL
    device_id TEXT NOT NULL,
    season_id TEXT,
    action TEXT NOT NULL,
    tx_id TEXT NOT NULL,
    created_at TIMESTAMPTZ DEFAULT CURRENT_TIMESTAMP
);

-- INDEXES
CREATE INDEX idx_blockchain_device ON blockchain_history(device_id);
CREATE INDEX idx_pump_device ON pump_calibration(device_id);
