ALTER TABLE system_events
 ADD COLUMN IF NOT EXISTS event_type TEXT NOT NULL DEFAULT 'legacy.event',
 ADD COLUMN IF NOT EXISTS actor_kind TEXT NOT NULL DEFAULT 'unknown',
 ADD COLUMN IF NOT EXISTS actor_id TEXT,
 ADD COLUMN IF NOT EXISTS received_at TIMESTAMPTZ NOT NULL DEFAULT CURRENT_TIMESTAMP,
 ADD COLUMN IF NOT EXISTS resolved_by TEXT;
ALTER TABLE system_events ADD CONSTRAINT chk_system_event_level_p1_6 CHECK (level IN ('info','success','warning','error','critical'));
ALTER TABLE system_events ADD CONSTRAINT chk_system_event_category_p1_6 CHECK (category IN ('alert','automation','calibration','device','dosing','sensor','system','user_action','water','script_alert','audit','end_season','recipe_override','sensors','status'));
ALTER TABLE system_events ADD CONSTRAINT chk_system_event_source_p1_6 CHECK (source IN ('user','controller','sensor','fsm','automation','watchdog','backend','system','rule','ai_supervisor','api','command_lifecycle','unknown'));
ALTER TABLE system_events ADD CONSTRAINT chk_system_event_actor_kind_p1_6 CHECK (actor_kind IN ('user','service','device','system','unknown'));
ALTER TABLE system_events ADD CONSTRAINT chk_system_event_event_type_p1_6 CHECK (length(event_type) BETWEEN 1 AND 128 AND event_type ~ '^[a-z0-9][a-z0-9_.-]*$');
CREATE INDEX IF NOT EXISTS idx_system_events_journal_device_order ON system_events(device_id,timestamp DESC,id DESC);
CREATE INDEX IF NOT EXISTS idx_system_events_journal_unresolved ON system_events(device_id,timestamp DESC,id DESC) WHERE resolved_at IS NULL;
CREATE OR REPLACE FUNCTION system_events_enforce_immutable() RETURNS trigger LANGUAGE plpgsql AS $$ BEGIN IF TG_OP='DELETE' THEN IF current_setting('hydragrow.retention_delete',true)='on' THEN RETURN OLD; END IF; RAISE EXCEPTION 'system_events are append-only'; END IF; IF NEW.id IS DISTINCT FROM OLD.id OR NEW.device_id IS DISTINCT FROM OLD.device_id OR NEW.level IS DISTINCT FROM OLD.level OR NEW.category IS DISTINCT FROM OLD.category OR NEW.title IS DISTINCT FROM OLD.title OR NEW.message IS DISTINCT FROM OLD.message OR NEW.reason IS DISTINCT FROM OLD.reason OR NEW.metadata IS DISTINCT FROM OLD.metadata OR NEW.timestamp IS DISTINCT FROM OLD.timestamp OR NEW.source IS DISTINCT FROM OLD.source OR NEW.primary_reason_code IS DISTINCT FROM OLD.primary_reason_code OR NEW.event_type IS DISTINCT FROM OLD.event_type OR NEW.actor_kind IS DISTINCT FROM OLD.actor_kind OR NEW.actor_id IS DISTINCT FROM OLD.actor_id OR NEW.received_at IS DISTINCT FROM OLD.received_at THEN RAISE EXCEPTION 'system_events occurrence fields are immutable'; END IF; RETURN NEW; END; $$;
DROP TRIGGER IF EXISTS trg_system_events_immutable ON system_events;
CREATE TRIGGER trg_system_events_immutable BEFORE UPDATE OR DELETE ON system_events FOR EACH ROW EXECUTE FUNCTION system_events_enforce_immutable();
