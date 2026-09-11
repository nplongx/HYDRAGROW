-- hydragrow-backend/migrations/20260911000000_restore_action_command_script_kind.sql
--
-- F5: loại 'config_override' bị khai tử ở tầng IR/UI/backend (Flow Config·Overwrite
-- giờ là kind 'alert' kèm khối configOverwrite trong ir_json). Khôi phục CHECK
-- về đúng 3 kind (đã bị nới lỏng bởi 20260830090100 + các bản sau).
--
-- Bước UPDATE là defensive: trên DB chỉ có 3 kind hợp lệ thì đây là no-op; nếu
-- còn sót row config_override cũ (chưa được frontend normalise) thì đưa về
-- 'alert' để không vi phạm CHECK vừa khôi phục (row đó tự vô hại — rhai alert
-- không có configOverwrite thì không áp override nào).

UPDATE user_scripts
   SET kind = 'alert'
 WHERE kind NOT IN ('alert', 'recipe_override', 'action_command');

ALTER TABLE user_scripts DROP CONSTRAINT IF EXISTS user_scripts_kind_check;
ALTER TABLE user_scripts ADD CONSTRAINT user_scripts_kind_check
  CHECK (kind IN ('alert', 'recipe_override', 'action_command'));

COMMENT ON COLUMN user_scripts.kind IS
  'alert: nhận ScriptSensorInput, trả AlertOutput (có thể kèm configOverwrite) | recipe_override: nhận ScriptFsmInput, trả StageOverride | action_command: nhận ScriptActionInput, trả ActionCommandOutput (bắt buộc qua safety gate trước khi publish MQTT)';