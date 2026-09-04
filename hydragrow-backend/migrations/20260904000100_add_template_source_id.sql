-- Migration: Cột template_source_id và template_overrides cho user_scripts (Phase 6)
ALTER TABLE user_scripts ADD COLUMN IF NOT EXISTS template_source_id UUID NULL REFERENCES user_scripts(id) ON DELETE SET NULL;
ALTER TABLE user_scripts ADD COLUMN IF NOT EXISTS template_overrides JSONB NOT NULL DEFAULT '{}'::jsonb;
CREATE INDEX IF NOT EXISTS idx_user_scripts_template_source ON user_scripts (template_source_id) WHERE template_source_id IS NOT NULL;
