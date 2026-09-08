-- hydragrow-backend/migrations/20260907090000_add_priority_and_value_to_flow_config_overrides.sql

-- override_value: giá trị THẬT đã ghi đè (trước migration này, API audit log
-- phải giả lập bằng cách lặp lại original_value — xem
-- hydragrow-backend/src/api/script.rs). Default '' chỉ để backfill an toàn
-- các dòng lịch sử đã có sẵn; mọi INSERT mới đều truyền giá trị thật.
ALTER TABLE flow_config_overrides
    ADD COLUMN IF NOT EXISTS override_value TEXT NOT NULL DEFAULT '',
    ADD COLUMN IF NOT EXISTS priority INT NOT NULL DEFAULT 0,
    ADD COLUMN IF NOT EXISTS clamped BOOLEAN NOT NULL DEFAULT false;

ALTER TABLE flow_config_overrides ALTER COLUMN override_value DROP DEFAULT;
