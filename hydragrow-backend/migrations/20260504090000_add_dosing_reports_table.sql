CREATE TABLE IF NOT EXISTS dosing_reports (
    id SERIAL PRIMARY KEY,
    device_id TEXT NOT NULL,
    season_id TEXT,
    pump_a_ml REAL NOT NULL DEFAULT 0,
    pump_b_ml REAL NOT NULL DEFAULT 0,
    ph_up_ml REAL NOT NULL DEFAULT 0,
    ph_down_ml REAL NOT NULL DEFAULT 0,
    payload JSONB NOT NULL,
    created_at TIMESTAMPTZ NOT NULL DEFAULT CURRENT_TIMESTAMP
);

CREATE INDEX IF NOT EXISTS idx_dosing_reports_device_season_created
    ON dosing_reports(device_id, season_id, created_at DESC);
