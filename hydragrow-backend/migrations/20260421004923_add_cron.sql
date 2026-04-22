-- 1. Bảng device_config
ALTER TABLE device_config DROP COLUMN IF EXISTS pump_a_capacity_ml_per_sec;
ALTER TABLE device_config DROP COLUMN IF EXISTS pump_b_capacity_ml_per_sec;

-- 2. Bảng water_config
ALTER TABLE water_config 
    ADD COLUMN IF NOT EXISTS water_change_cron VARCHAR(50) DEFAULT '0 0 7 * * SUN',
    DROP COLUMN IF EXISTS water_change_interval_sec;

-- 3. Bảng dosing_calibration
ALTER TABLE dosing_calibration 
    -- Đổi tên cột dùng chung cũ
    RENAME COLUMN dosing_pump_capacity_ml_per_sec TO pump_a_capacity_ml_per_sec;

-- Thêm các cột capacity mới (Nếu Rename ở trên đã tạo pump_a thì ko add lại nữa)
ALTER TABLE dosing_calibration 
    ADD COLUMN IF NOT EXISTS pump_b_capacity_ml_per_sec REAL DEFAULT 1.2,
    ADD COLUMN IF NOT EXISTS pump_ph_up_capacity_ml_per_sec REAL DEFAULT 1.2,
    ADD COLUMN IF NOT EXISTS pump_ph_down_capacity_ml_per_sec REAL DEFAULT 1.2,
    
    -- Cập nhật cron
    ADD COLUMN IF NOT EXISTS scheduled_dosing_cron VARCHAR(50) DEFAULT '0 0 8 * * *',
    DROP COLUMN IF EXISTS scheduled_dosing_interval_sec;
