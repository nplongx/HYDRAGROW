ALTER TABLE users ADD COLUMN IF NOT EXISTS role TEXT NOT NULL DEFAULT 'viewer';
ALTER TABLE users ADD COLUMN IF NOT EXISTS preferences JSONB NOT NULL DEFAULT '{}'::jsonb;

DO $$
BEGIN
    IF NOT EXISTS (
        SELECT 1 FROM pg_constraint WHERE conname = 'chk_users_role'
    ) THEN
        ALTER TABLE users ADD CONSTRAINT chk_users_role CHECK (role IN ('admin', 'operator', 'viewer'));
    END IF;
END $$;

-- Backfill role từ scopes hiện có (chỉ áp dụng cho user đã tồn tại).
UPDATE users SET role = CASE
    WHEN scopes && ARRAY['device:admin', 'admin', '*'] THEN 'admin'
    WHEN scopes && ARRAY['write:config', 'control:pump', 'control:emergency',
                        'device:ota', 'device:network', 'script:write', 'recipe:write'] THEN 'operator'
    ELSE 'viewer'
END
WHERE scopes IS NOT NULL;
