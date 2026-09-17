CREATE TABLE IF NOT EXISTS configuration_sync (
    device_id TEXT PRIMARY KEY REFERENCES device_config(device_id) ON DELETE CASCADE,
    config_version BIGINT NOT NULL,
    desired_controller_config JSONB NOT NULL,
    desired_sensor_config JSONB NOT NULL,
    controller_state TEXT NOT NULL DEFAULT 'pending',
    sensor_state TEXT NOT NULL DEFAULT 'pending',
    controller_attempts INTEGER NOT NULL DEFAULT 0,
    sensor_attempts INTEGER NOT NULL DEFAULT 0,
    controller_last_attempt_at TIMESTAMPTZ,
    sensor_last_attempt_at TIMESTAMPTZ,
    controller_applied_at TIMESTAMPTZ,
    sensor_applied_at TIMESTAMPTZ,
    last_error TEXT,
    updated_at TIMESTAMPTZ NOT NULL DEFAULT CURRENT_TIMESTAMP,
    CONSTRAINT chk_configuration_sync_controller_state
        CHECK (controller_state IN ('pending', 'published', 'applied', 'failed')),
    CONSTRAINT chk_configuration_sync_sensor_state
        CHECK (sensor_state IN ('pending', 'published', 'applied', 'failed')),
    CONSTRAINT chk_configuration_sync_attempts_nonnegative
        CHECK (controller_attempts >= 0 AND sensor_attempts >= 0)
);

CREATE INDEX IF NOT EXISTS idx_configuration_sync_pending
    ON configuration_sync (updated_at)
    WHERE controller_state IN ('pending', 'published')
       OR sensor_state IN ('pending', 'published');
