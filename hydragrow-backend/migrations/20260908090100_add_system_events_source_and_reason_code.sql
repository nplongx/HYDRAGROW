ALTER TABLE system_events ADD COLUMN IF NOT EXISTS source TEXT NOT NULL DEFAULT 'rule';
ALTER TABLE system_events ADD CONSTRAINT chk_system_event_source CHECK (source IN ('rule', 'watchdog', 'ai_supervisor'));
ALTER TABLE system_events ADD COLUMN IF NOT EXISTS primary_reason_code TEXT;
ALTER TABLE system_events ADD CONSTRAINT chk_system_event_primary_reason_code CHECK (primary_reason_code IS NULL OR primary_reason_code IN ('ec_out_of_range','ec_degrading','ph_out_of_range','ph_degrading','water_level_out_of_range','water_level_degrading','temp_out_of_range','temp_degrading','leak_suspected','sensor_fault_suspected','dosing_ineffective','unexplained_anomaly','topic_stale_controller_status'));
CREATE INDEX IF NOT EXISTS idx_system_events_dedup ON system_events (device_id, source, primary_reason_code, timestamp DESC);
