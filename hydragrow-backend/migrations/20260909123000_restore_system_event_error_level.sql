-- Script alerts can legitimately persist with level = 'error'.
-- Restore 'error' after 20260907120001_drop_unused_system_event_error_level.sql
-- removed it from chk_system_event_level.
ALTER TABLE system_events
    DROP CONSTRAINT chk_system_event_level;

ALTER TABLE system_events
    ADD CONSTRAINT chk_system_event_level
        CHECK (level IN ('info', 'success', 'warning', 'critical', 'error'));
