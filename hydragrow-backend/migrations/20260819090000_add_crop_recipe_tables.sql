-- =============================================================================
-- Migration: Add crop recipe tables and active recipe assignment tracking
-- =============================================================================

-- Recipes can be reusable templates and may optionally be scoped to a season.
CREATE TABLE IF NOT EXISTS crop_recipes (
    id TEXT PRIMARY KEY NOT NULL,
    season_id TEXT,
    name TEXT NOT NULL,
    crop_type TEXT,
    description TEXT,
    metadata JSONB NOT NULL DEFAULT '{}'::jsonb,
    created_at TIMESTAMPTZ NOT NULL DEFAULT CURRENT_TIMESTAMP,
    updated_at TIMESTAMPTZ NOT NULL DEFAULT CURRENT_TIMESTAMP,
    CONSTRAINT chk_crop_recipes_name_not_empty
        CHECK (length(trim(name)) > 0),
    CONSTRAINT chk_crop_recipes_metadata_object
        CHECK (jsonb_typeof(metadata) = 'object'),
    FOREIGN KEY (season_id) REFERENCES crop_seasons(id) ON DELETE SET NULL
);

-- Stage-level targets for each recipe. target_config keeps EC/pH/water/light/etc.
-- extensible without requiring a migration for every target type.
CREATE TABLE IF NOT EXISTS crop_recipe_stages (
    id TEXT PRIMARY KEY NOT NULL,
    recipe_id TEXT NOT NULL,
    stage_order INTEGER NOT NULL,
    name TEXT NOT NULL,
    duration_days INTEGER,
    target_config JSONB NOT NULL DEFAULT '{}'::jsonb,
    notes TEXT,
    created_at TIMESTAMPTZ NOT NULL DEFAULT CURRENT_TIMESTAMP,
    updated_at TIMESTAMPTZ NOT NULL DEFAULT CURRENT_TIMESTAMP,
    CONSTRAINT chk_crop_recipe_stages_name_not_empty
        CHECK (length(trim(name)) > 0),
    CONSTRAINT chk_crop_recipe_stages_stage_order_positive
        CHECK (stage_order > 0),
    CONSTRAINT chk_crop_recipe_stages_duration_days_positive
        CHECK (duration_days IS NULL OR duration_days > 0),
    CONSTRAINT chk_crop_recipe_stages_target_config_object
        CHECK (jsonb_typeof(target_config) = 'object'),
    CONSTRAINT uq_crop_recipe_stages_recipe_order
        UNIQUE (recipe_id, stage_order),
    FOREIGN KEY (recipe_id) REFERENCES crop_recipes(id) ON DELETE CASCADE
);

-- Tracks the active recipe/stage currently applied to a device and season.
CREATE TABLE IF NOT EXISTS device_active_recipes (
    id TEXT PRIMARY KEY NOT NULL,
    device_id TEXT NOT NULL,
    season_id TEXT,
    recipe_id TEXT NOT NULL,
    current_stage_id TEXT,
    applied_targets JSONB NOT NULL DEFAULT '{}'::jsonb,
    activated_at TIMESTAMPTZ NOT NULL DEFAULT CURRENT_TIMESTAMP,
    updated_at TIMESTAMPTZ NOT NULL DEFAULT CURRENT_TIMESTAMP,
    CONSTRAINT chk_device_active_recipes_applied_targets_object
        CHECK (jsonb_typeof(applied_targets) = 'object'),
    CONSTRAINT uq_device_active_recipes_device
        UNIQUE (device_id),
    FOREIGN KEY (device_id) REFERENCES device_config(device_id) ON DELETE CASCADE,
    FOREIGN KEY (season_id) REFERENCES crop_seasons(id) ON DELETE SET NULL,
    FOREIGN KEY (recipe_id) REFERENCES crop_recipes(id) ON DELETE CASCADE,
    FOREIGN KEY (current_stage_id) REFERENCES crop_recipe_stages(id) ON DELETE SET NULL
);

-- Required lookup indexes.
CREATE INDEX IF NOT EXISTS idx_crop_recipes_season_id
    ON crop_recipes(season_id);

CREATE INDEX IF NOT EXISTS idx_crop_recipe_stages_recipe_id
    ON crop_recipe_stages(recipe_id);

CREATE INDEX IF NOT EXISTS idx_device_active_recipes_recipe_id
    ON device_active_recipes(recipe_id);

CREATE INDEX IF NOT EXISTS idx_device_active_recipes_season_id
    ON device_active_recipes(season_id);

CREATE INDEX IF NOT EXISTS idx_device_active_recipes_device_id
    ON device_active_recipes(device_id);

-- Recipe lifecycle events can be stored in system_events.metadata. Ensure the
-- column remains JSONB and add expression indexes for common recipe filters.
ALTER TABLE system_events
    ADD COLUMN IF NOT EXISTS metadata JSONB;

ALTER TABLE system_events
    ALTER COLUMN metadata TYPE JSONB USING metadata::JSONB;

CREATE INDEX IF NOT EXISTS idx_system_events_metadata_recipe_id
    ON system_events ((metadata ->> 'recipe_id'));

CREATE INDEX IF NOT EXISTS idx_system_events_metadata_season_id
    ON system_events ((metadata ->> 'season_id'));
