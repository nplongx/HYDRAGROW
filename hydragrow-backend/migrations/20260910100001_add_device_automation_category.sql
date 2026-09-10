ALTER TABLE system_events
    DROP CONSTRAINT chk_system_event_category;

ALTER TABLE system_events
    ADD CONSTRAINT chk_system_event_category
        CHECK (category IN (
            'system', 'dosing', 'water',
            'alert', 'calibration', 'sensor', 'user_action',
            'script_alert', 'device', 'automation'
        ));