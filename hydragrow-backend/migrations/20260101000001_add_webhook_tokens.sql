CREATE TABLE webhook_tokens (
    id          UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    device_id   TEXT NOT NULL REFERENCES device_configs(device_id) ON DELETE CASCADE,
    label       TEXT NOT NULL,
    token_hash  TEXT NOT NULL UNIQUE,
    created_at  TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    last_used_at TIMESTAMPTZ,
    is_active   BOOLEAN NOT NULL DEFAULT TRUE
);
COMMENT ON TABLE webhook_tokens IS
    'Một token per integration (Zapier, Home Assistant) thay vì dùng chung root API key. '
    'Có thể thu hồi từng token mà không ảnh hưởng các integration khác.';
