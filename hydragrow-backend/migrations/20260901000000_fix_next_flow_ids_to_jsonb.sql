-- Fix: next_flow_ids được khai báo là TEXT trong migration 20260830200000
-- nhưng Rust model dùng #[sqlx(json)] kỳ vọng JSONB.
-- Tất cả giá trị hiện tại là JSON arrays hợp lệ ('[]' hoặc '["id1","id2"]')
-- nên USING cast an toàn.

-- Drop existing default first before changing type to JSONB
ALTER TABLE user_scripts
ALTER COLUMN next_flow_ids DROP DEFAULT;

ALTER TABLE user_scripts
ALTER COLUMN next_flow_ids TYPE JSONB
USING next_flow_ids::jsonb;

-- Cập nhật DEFAULT cho nhất quán với kiểu mới
ALTER TABLE user_scripts
ALTER COLUMN next_flow_ids SET DEFAULT '[]'::jsonb;

COMMENT ON COLUMN user_scripts.next_flow_ids IS
'Danh sách script IDs sẽ được kích hoạt sau khi script này thực thi thành công. JSONB array of UUID strings. Vắng / [] = Flow độc lập.';
