-- hydragrow-backend/migrations/20260818133600_undo_ec_to_tds_constraints.sql
-- Migration Undo: Khôi phục ràng buộc về chuẩn EC (mS/cm) nguyên bản

-- Chuẩn hóa dữ liệu nếu đã lỡ lưu giá trị PPM lớn
UPDATE device_config
SET ec_target = 1.5, ec_tolerance = 0.05
WHERE ec_target > 10.0;

UPDATE safety_config
SET min_ec_limit = 0.5,
max_ec_limit = 3.5,
max_ec_delta = 0.5,
ec_ack_threshold = 0.05
WHERE max_ec_limit > 10.0;

UPDATE dosing_calibration
SET ec_gain_per_ml = 0.015
WHERE ec_gain_per_ml > 5.0;

UPDATE sensor_calibration
SET ec_offset = 0.0
WHERE ec_offset < -5.0 OR ec_offset > 5.0;

-- Khôi phục ràng buộc device_config
ALTER TABLE device_config
DROP CONSTRAINT IF EXISTS chk_device_config_ec_target,
DROP CONSTRAINT IF EXISTS chk_device_config_ec_tolerance;

ALTER TABLE device_config
ADD CONSTRAINT chk_device_config_ec_target
CHECK (ec_target > 0 AND ec_target <= 10.0),
ADD CONSTRAINT chk_device_config_ec_tolerance
CHECK (ec_tolerance >= 0 AND ec_tolerance < ec_target);

-- Khôi phục ràng buộc safety_config
ALTER TABLE safety_config
DROP CONSTRAINT IF EXISTS chk_safety_ec_limits,
DROP CONSTRAINT IF EXISTS chk_safety_ec_delta,
DROP CONSTRAINT IF EXISTS chk_safety_ec_ack_threshold;

ALTER TABLE safety_config
ADD CONSTRAINT chk_safety_ec_limits
CHECK (min_ec_limit >= 0 AND max_ec_limit > min_ec_limit),
ADD CONSTRAINT chk_safety_ec_delta
CHECK (max_ec_delta > 0 AND max_ec_delta <= 5.0),
ADD CONSTRAINT chk_safety_ec_ack_threshold
CHECK (ec_ack_threshold > 0 AND ec_ack_threshold <= 2.0);

-- Khôi phục ràng buộc dosing_calibration
ALTER TABLE dosing_calibration
DROP CONSTRAINT IF EXISTS chk_dosing_cal_ec_gain;

ALTER TABLE dosing_calibration
ADD CONSTRAINT chk_dosing_cal_ec_gain
CHECK (ec_gain_per_ml > 0 AND ec_gain_per_ml <= 5.0);

-- Khôi phục ràng buộc sensor_calibration
ALTER TABLE sensor_calibration
DROP CONSTRAINT IF EXISTS chk_sensor_cal_ec_offset;

ALTER TABLE sensor_calibration
ADD CONSTRAINT chk_sensor_cal_ec_offset
CHECK (ec_offset BETWEEN -5.0 AND 5.0);
