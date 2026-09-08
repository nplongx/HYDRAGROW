CREATE TABLE IF NOT EXISTS flow_execution_log (
  id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
  script_id UUID NOT NULL,
  device_id TEXT NOT NULL REFERENCES device_config(device_id) ON DELETE CASCADE,
  status TEXT NOT NULL CHECK (status IN ('success', 'error')),
  error_message TEXT,
  created_at TIMESTAMPTZ NOT NULL DEFAULT NOW()
);

CREATE INDEX IF NOT EXISTS idx_flow_execution_log_device_recent
  ON flow_execution_log (device_id, created_at DESC);
