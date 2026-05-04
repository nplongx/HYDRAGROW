-- Migration mới: đảm bảo index hiệu quả
CREATE INDEX IF NOT EXISTS idx_system_events_category_device 
  ON system_events(device_id, category, timestamp DESC);
