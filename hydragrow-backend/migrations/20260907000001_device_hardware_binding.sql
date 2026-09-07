ALTER TABLE device_ownership
  ADD COLUMN IF NOT EXISTS hardware_id TEXT;
