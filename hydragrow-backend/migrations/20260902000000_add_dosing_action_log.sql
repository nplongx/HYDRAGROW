CREATE TABLE IF NOT EXISTS dosing_action_log (
    id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    device_id TEXT NOT NULL REFERENCES device_config(device_id) ON DELETE CASCADE,
    pump TEXT NOT NULL,
    dose_ml REAL NOT NULL,
    dosed_at TIMESTAMPTZ NOT NULL DEFAULT NOW()
);

CREATE INDEX IF NOT EXISTS idx_dosing_action_log_device_dosed ON dosing_action_log(device_id, dosed_at);
