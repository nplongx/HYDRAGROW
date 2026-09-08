CREATE TABLE device_topic_last_seen (
    device_id TEXT NOT NULL,
    topic_category TEXT NOT NULL,
    last_seen_at TIMESTAMPTZ NOT NULL,
    PRIMARY KEY (device_id, topic_category)
);

COMMENT ON TABLE device_topic_last_seen IS 'Design spec section 6.1: LWT-independent per-topic last-seen timestamps, updated by MQTT handlers';
