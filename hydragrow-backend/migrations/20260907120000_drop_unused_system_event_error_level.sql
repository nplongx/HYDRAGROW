-- LogLevel (Rust) chỉ bao giờ serialize ra 'info' | 'success' | 'warning' | 'critical'
-- khi ghi vào system_events (xem hydragrow-shared/src/log.rs). Giữ 'error' trong
-- ràng buộc chỉ tạo ảo giác an toàn: không có INSERT nào dùng nó, và frontend
-- (EventLogCard) không có style riêng cho giá trị này.
ALTER TABLE system_events
    DROP CONSTRAINT chk_system_event_level;

ALTER TABLE system_events
    ADD CONSTRAINT chk_system_event_level
        CHECK (level IN ('info', 'success', 'warning', 'critical'));
