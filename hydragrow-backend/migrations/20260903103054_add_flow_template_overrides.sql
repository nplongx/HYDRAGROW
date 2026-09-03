CREATE TABLE flow_template_overrides (
    id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    source_script_id UUID NOT NULL REFERENCES user_scripts(id) ON DELETE CASCADE,
    target_device_id TEXT NOT NULL REFERENCES device_config(device_id) ON DELETE CASCADE,
    -- override_script_id: bản sao user_scripts thật cho target_device_id, có
    -- ir_json riêng (threshold khác) nhưng "biết" mình sinh ra từ source nào để
    -- khi source đổi, hỏi người dùng có đồng bộ phần CHƯA override hay không.
    override_script_id UUID NOT NULL REFERENCES user_scripts(id) ON DELETE CASCADE,
    -- Danh sách field trong ir_json mà người dùng đã tự sửa cho device này —
    -- dùng để "chỉ đồng bộ phần chưa override" như Figma frame 09 yêu cầu.
    overridden_fields JSONB NOT NULL DEFAULT '[]'::jsonb,
    created_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    updated_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    UNIQUE (source_script_id, target_device_id)
);
COMMENT ON TABLE flow_template_overrides IS
'Liên kết 1 Flow gốc (source_script_id) với các bản sao đã áp cho thiết bị khác (override_script_id). overridden_fields ghi field nào KHÔNG được ghi đè khi đồng bộ lại từ source — xem AUTOMATION-008 (Figma frame 09).';
