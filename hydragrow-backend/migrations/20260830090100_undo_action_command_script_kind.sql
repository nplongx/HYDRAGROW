-- hydragrow-backend/migrations/20260830090100_undo_action_command_script_kind.sql
ALTER TABLE user_scripts DROP CONSTRAINT IF EXISTS user_scripts_kind_check;
ALTER TABLE user_scripts ADD CONSTRAINT user_scripts_kind_check
  CHECK (kind IN ('alert', 'recipe_override'));
