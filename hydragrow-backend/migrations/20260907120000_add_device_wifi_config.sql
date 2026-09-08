-- Metadata cấu hình WiFi của thiết bị (KHÔNG bao gồm mật khẩu — mật khẩu chỉ
-- tồn tại trên NVS của ESP32, không bao giờ được ghi vào DB). Dùng để hiển
-- thị cho user biết thiết bị đang được cấu hình dùng (các) mạng nào.
CREATE TABLE IF NOT EXISTS device_wifi_config (
    id             BIGSERIAL    PRIMARY KEY,
    device_id      TEXT         NOT NULL,
    ssid           TEXT         NOT NULL,
    priority       SMALLINT     NOT NULL,
    config_version BIGINT       NOT NULL,
    updated_at     TIMESTAMPTZ  NOT NULL DEFAULT CURRENT_TIMESTAMP
);

CREATE INDEX IF NOT EXISTS idx_device_wifi_config_device ON device_wifi_config(device_id);
