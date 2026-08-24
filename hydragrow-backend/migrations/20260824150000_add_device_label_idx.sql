-- hydragrow-backend/migrations/2026-08-24-150000_add_device_label_idx.sql
-- Tối ưu tìm kiếm label và đảm bảo claimed_at có index
CREATE INDEX IF NOT EXISTS idx_device_ownership_claimed
    ON device_ownership(user_id, claimed_at);
