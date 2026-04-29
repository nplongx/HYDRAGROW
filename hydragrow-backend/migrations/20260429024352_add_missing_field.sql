ALTER TABLE dosing_calibration
  ADD COLUMN IF NOT EXISTS dosing_min_pwm_percent  INTEGER NOT NULL DEFAULT 35,
  ADD COLUMN IF NOT EXISTS pump_a_min_pwm_percent  INTEGER,
  ADD COLUMN IF NOT EXISTS pump_b_min_pwm_percent  INTEGER,
  ADD COLUMN IF NOT EXISTS pump_ph_up_min_pwm_percent  INTEGER,
  ADD COLUMN IF NOT EXISTS pump_ph_down_min_pwm_percent INTEGER,
  ADD COLUMN IF NOT EXISTS dosing_pulse_on_ms       INTEGER NOT NULL DEFAULT 250,
  ADD COLUMN IF NOT EXISTS dosing_pulse_off_ms      INTEGER NOT NULL DEFAULT 300,
  ADD COLUMN IF NOT EXISTS dosing_min_dose_ml       REAL    NOT NULL DEFAULT 0.4,
  ADD COLUMN IF NOT EXISTS dosing_max_pulse_count_per_cycle INTEGER NOT NULL DEFAULT 40;

ALTER TABLE water_config
  ADD COLUMN IF NOT EXISTS misting_temp_threshold          REAL   NOT NULL DEFAULT 30.0,
  ADD COLUMN IF NOT EXISTS high_temp_misting_on_duration_ms  BIGINT NOT NULL DEFAULT 15000,
  ADD COLUMN IF NOT EXISTS high_temp_misting_off_duration_ms BIGINT NOT NULL DEFAULT 60000;
