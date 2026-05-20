-- Add interaction matrix tracking fields for dynamic dosing calibration.
-- Idempotent via IF NOT EXISTS for safe re-runs across environments.
ALTER TABLE IF EXISTS dosing_calibration
    ADD COLUMN IF NOT EXISTS interaction_matrix JSONB,
    ADD COLUMN IF NOT EXISTS matrix_update_count INTEGER NOT NULL DEFAULT 0,
    ADD COLUMN IF NOT EXISTS matrix_is_warm BOOLEAN NOT NULL DEFAULT FALSE;
