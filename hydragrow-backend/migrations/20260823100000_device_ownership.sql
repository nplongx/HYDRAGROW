-- Bảng gán quyền sở hữu thiết bị cho người dùng (multi-tenant).
CREATE TABLE device_ownership (
    id         BIGSERIAL PRIMARY KEY,
    user_id    BIGINT       NOT NULL REFERENCES users(id) ON DELETE CASCADE,
    device_id  TEXT         NOT NULL,
    label      TEXT,                               -- tên người dùng đặt cho thiết bị
    claimed_at TIMESTAMPTZ  NOT NULL DEFAULT CURRENT_TIMESTAMP,
    UNIQUE (user_id, device_id)
);

CREATE INDEX idx_device_ownership_user    ON device_ownership(user_id);
CREATE INDEX idx_device_ownership_device  ON device_ownership(device_id);
