-- Add migration script here
-- Thêm các cột còn thiếu vào system_events
ALTER TABLE system_events
    ADD COLUMN IF NOT EXISTS category TEXT NOT NULL DEFAULT 'system',
    ADD COLUMN IF NOT EXISTS reason TEXT,
    ADD COLUMN IF NOT EXISTS metadata JSONB;
