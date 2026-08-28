CREATE TABLE IF NOT EXISTS user_scripts (
    id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    device_id TEXT NOT NULL REFERENCES devices(device_id) ON DELETE CASCADE,
    kind TEXT NOT NULL CHECK (kind IN ('alert', 'recipe_override')),
    name TEXT NOT NULL,
    source TEXT NOT NULL,
    enabled BOOLEAN NOT NULL DEFAULT TRUE,
    created_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    updated_at TIMESTAMPTZ NOT NULL DEFAULT NOW()
);

CREATE INDEX idx_user_scripts_device_kind
ON user_scripts (device_id, kind)
WHERE enabled = TRUE;

COMMENT ON COLUMN user_scripts.kind IS
'alert: nhận ScriptSensorInput, trả AlertOutput | recipe_override: nhận ScriptFsmInput, trả StageOverride';

COMMENT ON COLUMN user_scripts.source IS
'Rhai source code. Phải export hàm main(input) với kiểu phù hợp với kind.';
