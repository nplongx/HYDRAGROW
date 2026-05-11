-- Đảm bảo cột metadata là kiểu JSONB (nếu trước đó bạn để text hoặc json thường)
ALTER TABLE system_event ALTER COLUMN metadata TYPE JSONB USING metadata::JSONB;

-- Tạo GIN Index để tăng tốc độ tìm kiếm bên trong JSONB (Rất quan trọng cho API filter)
CREATE INDEX idx_system_event_metadata_cycle_id 
ON system_event USING GIN ((metadata -> 'cycle_id'));

-- Tạo thêm index cho event_type để tiện filter (Ví dụ: Tìm tất cả lỗi Sensor)
CREATE INDEX idx_system_event_metadata_event_type 
ON system_event USING GIN ((metadata -> 'event_type'));
