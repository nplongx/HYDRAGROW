-- Add migration script here

ALTER TABLE dosing_calibration
DROP COLUMN ec_gain_dynamic,
DROP COLUMN ph_up_dynamic,
DROP COLUMN ph_down_dynamic,
DROP COLUMN dynamic_sample_count,
DROP COLUMN dynamic_confidence,
DROP COLUMN last_dynamic_update,
DROP COLUMN dynamic_model_version;
