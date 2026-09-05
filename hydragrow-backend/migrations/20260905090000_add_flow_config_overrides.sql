CREATE TABLE IF NOT EXISTS flow_config_overrides (
    id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    script_id UUID NOT NULL,
    device_id TEXT NOT NULL REFERENCES device_config(device_id) ON DELETE CASCADE,
    config_key TEXT NOT NULL,
    original_value TEXT NOT NULL,
    applied_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    restored_at TIMESTAMPTZ
);

-- Dùng để: (a) tìm bản backup chưa khôi phục của 1 (script_id, config_key) khi
-- condition chuyển true -> false, và (b) orphan recovery quét toàn bộ hàng
-- restored_at IS NULL khi khởi động server.
CREATE INDEX IF NOT EXISTS idx_flow_config_overrides_unrestored
ON flow_config_overrides(script_id, config_key)
WHERE restored_at IS NULL;
