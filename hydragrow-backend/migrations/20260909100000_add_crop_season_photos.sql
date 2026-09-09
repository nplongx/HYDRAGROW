CREATE TABLE IF NOT EXISTS crop_season_photos (
    id TEXT PRIMARY KEY NOT NULL,
    season_id TEXT NOT NULL REFERENCES crop_seasons(id),
    device_id TEXT NOT NULL,
    cloudinary_public_id TEXT NOT NULL,
    cloudinary_url TEXT NOT NULL,
    day_offset INTEGER NOT NULL,
    created_at TIMESTAMPTZ NOT NULL DEFAULT CURRENT_TIMESTAMP
);

CREATE INDEX IF NOT EXISTS idx_crop_season_photos_season ON crop_season_photos (season_id, day_offset);
