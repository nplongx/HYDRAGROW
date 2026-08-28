-- hydragrow-backend/migrations/20260828150000_add_script_ir_json.sql
-- Lưu Automation IR (JSON) song song với Rhai source đã compile, để mở lại
-- Blockly/React Flow editor thấy đúng graph đã build ra script này.
-- NULL nghĩa là script được viết tay (không qua visual builder).
ALTER TABLE user_scripts
ADD COLUMN IF NOT EXISTS ir_json JSONB;
