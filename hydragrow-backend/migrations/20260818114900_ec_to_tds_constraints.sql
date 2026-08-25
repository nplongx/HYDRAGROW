-- hydragrow-backend/migrations/20260818114900_ec_to_tds_constraints.sql
-- Migration: Cập nhật ràng buộc EC -> TDS (PPM) cho toàn bộ hệ thống

-- 1. device_config
ALTER TABLE device_config
DROP CONSTRAINT IF EXISTS chk_device_config_ec_target,
DROP CONSTRAINT IF EXISTS chk_device_config_ec_tolerance;

ALTER TABLE device_config
ADD CONSTRAINT chk_device_config_ec_target
CHECK (ec_target > 0 AND ec_target <= 5000.0),
ADD CONSTRAINT chk_device_config_ec_tolerance
CHECK (ec_tolerance >= 0 AND ec_tolerance < ec_target);

-- 2. safety_config
ALTER TABLE safety_config
DROP CONSTRAINT IF EXISTS chk_safety_ec_limits,
DROP CONSTRAINT IF EXISTS chk_safety_ec_delta,
DROP CONSTRAINT IF EXISTS chk_safety_ec_ack_threshold;

ALTER TABLE safety_config
ADD CONSTRAINT chk_safety_ec_limits
CHECK (min_ec_limit >= 0 AND max_ec_limit > min_ec_limit AND max_ec_limit <= 5000.0),
ADD CONSTRAINT chk_safety_ec_delta
CHECK (max_ec_delta > 0 AND max_ec_delta <= 1500.0),
ADD CONSTRAINT chk_safety_ec_ack_threshold
CHECK (ec_ack_threshold > 0 AND ec_ack_threshold <= 500.0);

-- 3. dosing_calibration
ALTER TABLE dosing_calibration
DROP CONSTRAINT IF EXISTS chk_dosing_cal_ec_gain;

ALTER TABLE dosing_calibration
ADD CONSTRAINT chk_dosing_cal_ec_gain
CHECK (ec_gain_per_ml > 0 AND ec_gain_per_ml <= 500.0);

-- 4. sensor_calibration
ALTER TABLE sensor_calibration
DROP CONSTRAINT IF EXISTS chk_sensor_cal_ec_offset;

ALTER TABLE sensor_calibration
ADD CONSTRAINT chk_sensor_cal_ec_offset
CHECK (ec_offset BETWEEN -1000.0 AND 1000.0);
