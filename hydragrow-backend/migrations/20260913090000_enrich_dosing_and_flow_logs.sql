-- Enrich dosing_action_log with efficacy tracking
ALTER TABLE dosing_action_log ADD COLUMN IF NOT EXISTS ec_before REAL;
ALTER TABLE dosing_action_log ADD COLUMN IF NOT EXISTS ec_after REAL;
ALTER TABLE dosing_action_log ADD COLUMN IF NOT EXISTS ph_before REAL;
ALTER TABLE dosing_action_log ADD COLUMN IF NOT EXISTS ph_after REAL;
ALTER TABLE dosing_action_log ADD COLUMN IF NOT EXISTS cycle_id TEXT;
ALTER TABLE dosing_action_log ADD COLUMN IF NOT EXISTS triggered_by TEXT DEFAULT 'fsm_auto';

-- Enrich flow_execution_log with execution telemetry
ALTER TABLE flow_execution_log ADD COLUMN IF NOT EXISTS trigger_source TEXT DEFAULT 'sensor_data';
ALTER TABLE flow_execution_log ADD COLUMN IF NOT EXISTS duration_ms INTEGER;
