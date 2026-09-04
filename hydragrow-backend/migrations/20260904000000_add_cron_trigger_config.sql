-- Migration: Thêm cột cron_next_run_at phục vụ cron trigger (Phase 4)
ALTER TABLE user_scripts ADD COLUMN IF NOT EXISTS cron_next_run_at TIMESTAMPTZ NULL;
CREATE INDEX IF NOT EXISTS idx_user_scripts_cron_next_run ON user_scripts (cron_next_run_at)
  WHERE cron_next_run_at IS NOT NULL;
