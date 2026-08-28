-- hydragrow-backend/migrations/20260828150000_add_script_ir_json.sql
-- Create user_scripts table if it doesn't exist yet
CREATE TABLE IF NOT EXISTS user_scripts (
    id UUID PRIMARY KEY,
    device_id VARCHAR(64) NOT NULL,
    kind VARCHAR(32) NOT NULL,
    name VARCHAR(255) NOT NULL,
    source TEXT NOT NULL,
    enabled BOOLEAN NOT NULL DEFAULT TRUE,
    created_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    updated_at TIMESTAMPTZ NOT NULL DEFAULT NOW()
);

-- Index cho device_id để hỗ trợ query danh sách script của device
CREATE INDEX IF NOT EXISTS idx_user_scripts_device_id ON user_scripts(device_id);

-- Lưu Automation IR (JSON) song song với Rhai source đã compile, để mở lại
-- Blockly/React Flow editor thấy đúng graph đã build ra script này.
-- NULL nghĩa là script được viết tay (không qua visual builder).
ALTER TABLE user_scripts
ADD COLUMN IF NOT EXISTS ir_json JSONB;
