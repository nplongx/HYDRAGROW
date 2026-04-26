DO $$
BEGIN
    IF EXISTS (
        SELECT 1 FROM information_schema.columns
        WHERE table_name = 'sensor_calibration' AND column_name = 'is_ph_enabled'
    ) THEN
        ALTER TABLE sensor_calibration RENAME COLUMN is_ph_enabled TO enable_ph_sensor;
    END IF;

    IF EXISTS (
        SELECT 1 FROM information_schema.columns
        WHERE table_name = 'sensor_calibration' AND column_name = 'is_ec_enabled'
    ) THEN
        ALTER TABLE sensor_calibration RENAME COLUMN is_ec_enabled TO enable_ec_sensor;
    END IF;

    IF EXISTS (
        SELECT 1 FROM information_schema.columns
        WHERE table_name = 'sensor_calibration' AND column_name = 'is_temp_enabled'
    ) THEN
        ALTER TABLE sensor_calibration RENAME COLUMN is_temp_enabled TO enable_temp_sensor;
    END IF;

    IF EXISTS (
        SELECT 1 FROM information_schema.columns
        WHERE table_name = 'sensor_calibration' AND column_name = 'is_water_level_enabled'
    ) THEN
        ALTER TABLE sensor_calibration RENAME COLUMN is_water_level_enabled TO enable_water_level_sensor;
    END IF;
END $$;
