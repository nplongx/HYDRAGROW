-- hydragrow-backend/migrations/20260818153000_add_per_pump_step_ratios.sql
-- Tách step_ratio thành các cột riêng biệt cho từng bơm
-- và thêm các cột best_* để runtime learning lưu giá trị tốt nhất.

ALTER TABLE dosing_calibration
ADD COLUMN IF NOT EXISTS ec_a_step_ratio REAL NOT NULL DEFAULT 0.4,
ADD COLUMN IF NOT EXISTS ec_b_step_ratio REAL NOT NULL DEFAULT 0.4,
ADD COLUMN IF NOT EXISTS ph_up_step_ratio REAL NOT NULL DEFAULT 0.2,
ADD COLUMN IF NOT EXISTS ph_down_step_ratio REAL NOT NULL DEFAULT 0.2,
ADD COLUMN IF NOT EXISTS best_ec_a_ratio REAL NOT NULL DEFAULT 0.4,
ADD COLUMN IF NOT EXISTS best_ec_b_ratio REAL NOT NULL DEFAULT 0.4,
ADD COLUMN IF NOT EXISTS best_ph_up_ratio REAL NOT NULL DEFAULT 0.2,
ADD COLUMN IF NOT EXISTS best_ph_down_ratio REAL NOT NULL DEFAULT 0.2;
