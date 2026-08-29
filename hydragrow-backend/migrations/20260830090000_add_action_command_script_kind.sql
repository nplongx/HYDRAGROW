-- hydragrow-backend/migrations/20260830090000_add_action_command_script_kind.sql
ALTER TABLE user_scripts DROP CONSTRAINT IF EXISTS user_scripts_kind_check;
ALTER TABLE user_scripts ADD CONSTRAINT user_scripts_kind_check
  CHECK (kind IN ('alert', 'recipe_override', 'action_command'));

COMMENT ON COLUMN user_scripts.kind IS
  'alert: nhận ScriptSensorInput, trả AlertOutput | recipe_override: nhận ScriptFsmInput, trả StageOverride | action_command: nhận ScriptActionInput, trả ActionCommandOutput (bắt buộc qua safety gate trước khi publish MQTT)';
