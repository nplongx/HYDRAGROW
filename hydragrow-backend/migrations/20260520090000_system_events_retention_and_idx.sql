-- Retention policy job query (run via cron/pg_cron scheduler)
DELETE FROM system_events
WHERE timestamp < (EXTRACT(EPOCH FROM NOW() - INTERVAL '90 days') * 1000)::BIGINT;

-- Support cursor-based pagination by device + newest-first timestamp scans
CREATE INDEX IF NOT EXISTS idx_system_events_device_ts
  ON system_events(device_id, timestamp DESC);
