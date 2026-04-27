-- Add migration script here
ALTER TABLE sensor_calibration
ADD COLUMN ph_v10 REAL DEFAULT NULL,
ADD COLUMN ph_calibration_mode VARCHAR(20) DEFAULT '2-point';
