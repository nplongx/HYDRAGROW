ALTER TABLE device_config
  DROP COLUMN IF EXISTS temp_target,
  DROP COLUMN IF EXISTS temp_tolerance;

ALTER TABLE dosing_calibration
  DROP COLUMN IF EXISTS scheduled_dosing_enabled,
  DROP COLUMN IF EXISTS scheduled_dosing_cron,
  DROP COLUMN IF EXISTS scheduled_dose_a_ml,
  DROP COLUMN IF EXISTS scheduled_dose_b_ml;
