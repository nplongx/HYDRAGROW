CREATE TABLE commands (
    command_id TEXT PRIMARY KEY,
    idempotency_key TEXT,
    idempotency_scope TEXT,
    principal_kind TEXT NOT NULL,
    principal_id TEXT,
    service_key_label TEXT,
    session_id TEXT,
    user_id BIGINT REFERENCES users(id) ON DELETE SET NULL,
    device_id TEXT NOT NULL,
    action TEXT NOT NULL,
    request_payload JSONB NOT NULL DEFAULT '{}'::jsonb,
    requested_state BOOLEAN,
    requested_pwm INTEGER,
    pump_id TEXT,
    lifecycle TEXT NOT NULL CHECK (lifecycle IN ('REQUESTED','SENT','ACKNOWLEDGED','CONFIRMED','REJECTED','FAILED','TIMEOUT','UNKNOWN')),
    created_at TIMESTAMPTZ NOT NULL DEFAULT CURRENT_TIMESTAMP,
    authorized_at TIMESTAMPTZ NOT NULL DEFAULT CURRENT_TIMESTAMP,
    sent_at TIMESTAMPTZ,
    acknowledged_at TIMESTAMPTZ,
    confirmed_at TIMESTAMPTZ,
    terminal_at TIMESTAMPTZ,
    next_retry_at TIMESTAMPTZ,
    attempt_count INTEGER NOT NULL DEFAULT 0 CHECK (attempt_count >= 0),
    last_error TEXT,
    last_observed_at TIMESTAMPTZ,
    version BIGINT NOT NULL DEFAULT 0,
    retry_safe BOOLEAN NOT NULL DEFAULT TRUE
);

CREATE UNIQUE INDEX uq_commands_idempotency
    ON commands (idempotency_scope, idempotency_key)
    WHERE idempotency_key IS NOT NULL;
CREATE INDEX idx_commands_device_created ON commands (device_id, created_at DESC);
CREATE INDEX idx_commands_lifecycle_retry ON commands (lifecycle, next_retry_at);

CREATE TABLE command_lifecycle_events (
    id BIGSERIAL PRIMARY KEY,
    command_id TEXT NOT NULL REFERENCES commands(command_id) ON DELETE CASCADE,
    sequence_no INTEGER NOT NULL,
    from_lifecycle TEXT,
    lifecycle TEXT NOT NULL,
    device_id TEXT NOT NULL,
    reason TEXT,
    source TEXT NOT NULL,
    attempt_no INTEGER NOT NULL DEFAULT 0,
    occurred_at TIMESTAMPTZ NOT NULL DEFAULT CURRENT_TIMESTAMP,
    metadata JSONB NOT NULL DEFAULT '{}'::jsonb,
    UNIQUE (command_id, sequence_no)
);

CREATE INDEX idx_command_events_command ON command_lifecycle_events (command_id, sequence_no);
CREATE INDEX idx_command_events_device ON command_lifecycle_events (device_id, occurred_at DESC);
