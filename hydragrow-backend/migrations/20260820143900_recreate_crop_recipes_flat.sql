-- hydragrow-backend/migrations/20260820143900_recreate_crop_recipes_flat.sql
-- Xóa thiết kế JSONB cũ, tạo lại với cột phẳng tường minh.

DROP TABLE IF EXISTS device_active_recipes CASCADE;
DROP TABLE IF EXISTS crop_recipe_stages CASCADE;
DROP TABLE IF EXISTS crop_recipes CASCADE;

CREATE TABLE crop_recipes (
id TEXT PRIMARY KEY,
name TEXT NOT NULL,
crop TEXT NOT NULL DEFAULT 'general',
description TEXT,
created_at TIMESTAMPTZ NOT NULL DEFAULT NOW()
);

CREATE TABLE crop_recipe_stages (
id TEXT PRIMARY KEY,
recipe_id TEXT NOT NULL REFERENCES crop_recipes(id) ON DELETE CASCADE,
stage_order INT NOT NULL DEFAULT 1,
name TEXT NOT NULL,
duration_days INT NOT NULL DEFAULT 7,
ec_target REAL NOT NULL DEFAULT 1.4,
ec_tolerance REAL NOT NULL DEFAULT 0.1,
ph_target REAL NOT NULL DEFAULT 6.0,
ph_tolerance REAL NOT NULL DEFAULT 0.2,
nutrient_a_ratio REAL NOT NULL DEFAULT 1.0,
nutrient_b_ratio REAL NOT NULL DEFAULT 1.0,
water_level_target REAL NOT NULL DEFAULT 20.0,
water_change_interval_days INT,
water_change_drain_cm REAL,
auto_dilute_ec_trigger REAL,
misting_on_duration_ms INT NOT NULL DEFAULT 10000,
misting_off_duration_ms INT NOT NULL DEFAULT 180000,
max_dose_per_cycle_ml REAL
);

CREATE TABLE device_active_recipes (
id TEXT PRIMARY KEY,
device_id TEXT NOT NULL UNIQUE,
season_id TEXT NOT NULL,
recipe_id TEXT NOT NULL REFERENCES crop_recipes(id) ON DELETE CASCADE,
current_stage_id TEXT NOT NULL DEFAULT 'stage_1',
applied_at TIMESTAMPTZ NOT NULL DEFAULT NOW()
);
