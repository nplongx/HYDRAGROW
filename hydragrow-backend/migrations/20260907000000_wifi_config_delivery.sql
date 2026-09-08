CREATE TABLE IF NOT EXISTS device_wifi_config (
    device_id TEXT NOT NULL,
    ssid TEXT NOT NULL,
    priority SMALLINT NOT NULL DEFAULT 0,
    config_version BIGINT NOT NULL DEFAULT 0,
    updated_at TIMESTAMPTZ NOT NULL DEFAULT CURRENT_TIMESTAMP,
    PRIMARY KEY (device_id, ssid)
);

CREATE TABLE IF NOT EXISTS device_wifi_delivery (
    device_id TEXT PRIMARY KEY,
    config_version BIGINT NOT NULL,
    desired_state TEXT NOT NULL,
    last_result TEXT,
    updated_at TIMESTAMPTZ NOT NULL DEFAULT CURRENT_TIMESTAMP,
    applied_at TIMESTAMPTZ
);
