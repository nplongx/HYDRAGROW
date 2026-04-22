ALTER TABLE dosing_calibration 
ADD COLUMN scheduled_dosing_enabled BOOLEAN NOT NULL DEFAULT FALSE,
ADD COLUMN scheduled_dosing_interval_sec INTEGER NOT NULL DEFAULT 86400,
ADD COLUMN scheduled_dose_a_ml REAL NOT NULL DEFAULT 10.0,
ADD COLUMN scheduled_dose_b_ml REAL NOT NULL DEFAULT 10.0;
